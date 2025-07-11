use crate::config::AppConfig;
use crate::error::Result;
use crate::raft::{
    metrics::RaftMetricsCollector,
    network::{ConfluxNetworkFactory, NetworkConfig},
    store::{Store, StateMachineManager},
    types::*,
};
use openraft::{Config as RaftConfig, Raft};
use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info, warn};

/// Raft node configuration
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Node ID
    pub node_id: NodeId,
    /// Node address for network communication
    pub address: String,
    /// Raft configuration
    pub raft_config: RaftConfig,
    /// Network configuration
    pub network_config: NetworkConfig,
    /// Heartbeat interval in milliseconds (default: 150ms)
    pub heartbeat_interval: u64,
    /// Election timeout minimum in milliseconds (default: 300ms)
    pub election_timeout_min: u64,
    /// Election timeout maximum in milliseconds (default: 600ms)  
    pub election_timeout_max: u64,
    /// Resource limits configuration
    pub resource_limits: ResourceLimits,
}

/// Resource limits for client requests
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum requests per second per client
    pub max_requests_per_second: u32,
    /// Maximum concurrent requests
    pub max_concurrent_requests: u32,
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Maximum memory usage for pending requests (bytes)
    pub max_memory_usage: usize,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_requests_per_second: 100,
            max_concurrent_requests: 50,
            max_request_size: 1024 * 1024, // 1MB
            max_memory_usage: 50 * 1024 * 1024, // 50MB
            request_timeout_ms: 5000, // 5 seconds
        }
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: 1,
            address: "127.0.0.1:8080".to_string(),
            raft_config: RaftConfig::default(),
            network_config: NetworkConfig::default(),
            heartbeat_interval: 150,
            election_timeout_min: 300,
            election_timeout_max: 600,
            resource_limits: ResourceLimits::default(),
        }
    }
}

/// Client resource limiter for managing request limits
#[derive(Debug)]
pub struct ResourceLimiter {
    /// Resource limits configuration
    limits: ResourceLimits,
    /// Semaphore for concurrent request limiting
    concurrent_requests: Semaphore,
    /// Current memory usage for pending requests
    current_memory_usage: Arc<AtomicUsize>,
    /// Rate limiting state per client (if we had client IDs)
    rate_limit_state: RwLock<HashMap<String, RateLimitState>>,
    /// Global request count for metrics
    total_requests: AtomicU32,
    /// Rejected requests count
    rejected_requests: AtomicU32,
}

/// Rate limiting state for a client
#[derive(Debug, Clone)]
struct RateLimitState {
    /// Request count in current window
    request_count: u32,
    /// Window start time
    window_start: Instant,
}

impl ResourceLimiter {
    /// Create a new resource limiter
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            concurrent_requests: Semaphore::new(limits.max_concurrent_requests as usize),
            limits,
            current_memory_usage: Arc::new(AtomicUsize::new(0)),
            rate_limit_state: RwLock::new(HashMap::new()),
            total_requests: AtomicU32::new(0),
            rejected_requests: AtomicU32::new(0),
        }
    }

    /// Check if a request can be processed
    pub async fn check_request_allowed(&self, request_size: usize, client_id: Option<&str>) -> Result<RequestPermit<'_>> {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        // Check request size limit
        if request_size > self.limits.max_request_size {
            self.rejected_requests.fetch_add(1, Ordering::Relaxed);
            return Err(crate::error::ConfluxError::raft(format!(
                "Request size {} exceeds limit {}",
                request_size, self.limits.max_request_size
            )));
        }

        // Check memory usage limit
        let current_memory = self.current_memory_usage.load(Ordering::Relaxed);
        if current_memory + request_size > self.limits.max_memory_usage {
            self.rejected_requests.fetch_add(1, Ordering::Relaxed);
            return Err(crate::error::ConfluxError::raft(format!(
                "Memory usage limit exceeded: current={}, request={}, limit={}",
                current_memory, request_size, self.limits.max_memory_usage
            )));
        }

        // Check rate limit for client
        if let Some(client) = client_id {
            let mut state_map = self.rate_limit_state.write().await;
            let now = Instant::now();
            
            let client_state = state_map.entry(client.to_string()).or_insert_with(|| RateLimitState {
                request_count: 0,
                window_start: now,
            });

            // Reset window if it's been more than 1 second
            if now.duration_since(client_state.window_start) >= Duration::from_secs(1) {
                client_state.request_count = 0;
                client_state.window_start = now;
            }

            // Check rate limit
            if client_state.request_count >= self.limits.max_requests_per_second {
                self.rejected_requests.fetch_add(1, Ordering::Relaxed);
                return Err(crate::error::ConfluxError::raft(format!(
                    "Rate limit exceeded for client {}: {} requests/second",
                    client, client_state.request_count
                )));
            }

            client_state.request_count += 1;
        }

        // Try to acquire concurrent request permit
        match self.concurrent_requests.try_acquire() {
            Ok(permit) => {
                // Reserve memory for this request
                self.current_memory_usage.fetch_add(request_size, Ordering::Relaxed);
                
                Ok(RequestPermit {
                    _permit: permit,
                    request_size,
                    memory_tracker: self.current_memory_usage.clone(),
                })
            }
            Err(_) => {
                self.rejected_requests.fetch_add(1, Ordering::Relaxed);
                Err(crate::error::ConfluxError::raft(format!(
                    "Too many concurrent requests: limit={}",
                    self.limits.max_concurrent_requests
                )))
            }
        }
    }

    /// Get resource usage statistics
    pub fn get_resource_stats(&self) -> ResourceStats {
        ResourceStats {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            rejected_requests: self.rejected_requests.load(Ordering::Relaxed),
            current_memory_usage: self.current_memory_usage.load(Ordering::Relaxed),
            available_permits: self.concurrent_requests.available_permits(),
            max_concurrent_requests: self.limits.max_concurrent_requests as usize,
        }
    }

    /// Update resource limits
    pub fn update_limits(&mut self, new_limits: ResourceLimits) {
        self.limits = new_limits;
        // Note: Changing semaphore permits at runtime is complex
        // This is a simplified implementation
        warn!("Resource limits updated - some changes may require restart");
    }
}

/// RAII guard for request permission and resource tracking
pub struct RequestPermit<'a> {
    _permit: tokio::sync::SemaphorePermit<'a>,
    request_size: usize,
    memory_tracker: Arc<AtomicUsize>,
}

impl Drop for RequestPermit<'_> {
    fn drop(&mut self) {
        // Release memory when request is completed
        self.memory_tracker.fetch_sub(self.request_size, Ordering::Relaxed);
    }
}

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceStats {
    pub total_requests: u32,
    pub rejected_requests: u32,
    pub current_memory_usage: usize,
    pub available_permits: usize,
    pub max_concurrent_requests: usize,
}

/// Raft node implementation with integrated openraft::Raft instance
pub struct RaftNode {
    /// Node configuration
    config: NodeConfig,
    /// Storage instance
    store: Arc<Store>,
    /// Network factory
    network_factory: Arc<RwLock<ConfluxNetworkFactory>>,
    /// Current cluster members
    members: Arc<RwLock<BTreeSet<NodeId>>>,
    /// The actual Raft instance
    raft: Option<ConfluxRaft>,
    /// State machine manager handle
    state_machine_handle: Option<tokio::task::JoinHandle<()>>,
    /// Metrics collector
    metrics_collector: Arc<RaftMetricsCollector>,
    /// Resource limiter for client requests
    resource_limiter: Arc<ResourceLimiter>,
}

impl RaftNode {
    /// Create a new Raft node
    pub async fn new(config: NodeConfig, app_config: &AppConfig) -> Result<Self> {
        info!(
            "Creating Raft node {} at {}",
            config.node_id, config.address
        );

        // Create storage and get event receiver
        let (store, event_receiver) = Store::new(&app_config.storage.data_dir).await?;
        let store = Arc::new(store);

        // Start state machine manager
        let mut state_machine_manager = StateMachineManager::new(store.clone(), event_receiver);
        let state_machine_handle = tokio::spawn(async move {
            state_machine_manager.run().await;
        });

        // Create network factory
        let network_factory = Arc::new(RwLock::new(ConfluxNetworkFactory::new(
            config.network_config.clone(),
        )));

        // Initialize members with self
        let mut members = BTreeSet::new();
        members.insert(config.node_id);

        // Create metrics collector
        let metrics_collector = Arc::new(RaftMetricsCollector::new(config.node_id));

        // Create resource limiter
        let resource_limiter = Arc::new(ResourceLimiter::new(config.resource_limits.clone()));

        Ok(Self {
            config,
            store,
            network_factory,
            members: Arc::new(RwLock::new(members)),
            raft: None, // Will be initialized in start()
            state_machine_handle: Some(state_machine_handle),
            metrics_collector,
            resource_limiter,
        })
    }

    /// Get node ID
    pub fn node_id(&self) -> NodeId {
        self.config.node_id
    }

    /// Get node address
    pub fn address(&self) -> &str {
        &self.config.address
    }

    /// Get storage instance
    pub fn store(&self) -> Arc<Store> {
        self.store.clone()
    }

    /// Get metrics collector
    pub fn metrics_collector(&self) -> Arc<RaftMetricsCollector> {
        self.metrics_collector.clone()
    }

    /// Get resource limiter
    pub fn resource_limiter(&self) -> Arc<ResourceLimiter> {
        self.resource_limiter.clone()
    }

    /// Start the node and initialize Raft instance
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Raft node {}", self.config.node_id);

        // openraft 0.9 正确初始化方式：直接用 Arc<Store> 作为 storage
        let network_factory = {
            let factory = self.network_factory.read().await;
            factory.clone()
        };

        let mut raft_config = self.config.raft_config.clone();
        raft_config.heartbeat_interval = self.config.heartbeat_interval;
        raft_config.election_timeout_min = self.config.election_timeout_min;
        raft_config.election_timeout_max = self.config.election_timeout_max;

        // openraft 0.9 storage v2 不再使用 Adaptor
        // 直接使用 Store 作为 RaftLogStorage 和创建 ConfluxStateMachineWrapper
        let log_storage = self.store.clone();
        let state_machine = crate::raft::state_machine::ConfluxStateMachineWrapper::new(self.store.clone());

        // openraft 0.9 Raft::new 需要5个参数：node_id, config, network_factory, log_storage, state_machine
        match Raft::new(
            self.config.node_id,
            Arc::new(raft_config),
            network_factory,
            log_storage,
            state_machine,
        ).await {
            Ok(raft) => {
                self.raft = Some(raft);
                info!("Raft instance initialized successfully for node {}", self.config.node_id);
            }
            Err(e) => {
                error!("Failed to initialize Raft instance: {}", e);
                return Err(crate::error::ConfluxError::raft(format!("Raft initialization failed: {}", e)));
            }
        }

        // Initialize single-node cluster if needed
        if self.is_single_node_cluster().await {
            self.initialize_cluster().await?;
        }

        info!("Raft node {} started successfully", self.config.node_id);
        Ok(())
    }

    /// Get the Raft instance (if available)
    pub fn get_raft(&self) -> Option<&ConfluxRaft> {
        self.raft.as_ref()
    }

    /// Submit a client write request through Raft consensus with resource limits
    pub async fn client_write(&self, request: ClientRequest) -> Result<ClientWriteResponse> {
        let start_time = std::time::Instant::now();
        
        info!(
            "Processing client write through Raft consensus on node {}",
            self.config.node_id
        );

        // Calculate request size (simplified - in real implementation would serialize)
        let request_size = std::mem::size_of_val(&request) + 
                          request.command.estimate_size();

        // Check resource limits first
        let _permit = self.resource_limiter
            .check_request_allowed(request_size, None) // TODO: Add client ID when available
            .await?;

        let result = if let Some(ref raft) = self.raft {
            // Always route through Raft consensus - no fallback
            match raft.client_write(request).await {
                Ok(raft_response) => {
                    // The raft_response.data contains our ClientWriteResponse
                    Ok(raft_response.data)
                },
                Err(e) => {
                    error!("Raft client write failed: {}", e);
                    Err(crate::error::ConfluxError::raft(format!("Raft write failed: {}", e)))
                }
            }
        } else {
            // Return error if Raft is not initialized instead of fallback
            Err(crate::error::ConfluxError::raft("Raft not initialized - cannot process write requests"))
        };

        // Record request metrics
        let latency = start_time.elapsed();
        let success = result.is_ok();
        self.metrics_collector.record_request(latency, success).await;

        result
    }

    /// Stop the node (placeholder implementation)
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Raft node {}", self.config.node_id);
        debug!("Raft node {} stopped successfully", self.config.node_id);
        Ok(())
    }

    /// Get current cluster members
    pub async fn get_members(&self) -> BTreeSet<NodeId> {
        self.members.read().await.clone()
    }

    /// Add a new node to the cluster using Raft consensus
    pub async fn add_node(&self, node_id: NodeId, address: String) -> Result<()> {
        info!("Adding node {} at {} to cluster via Raft consensus", node_id, address);

        if let Some(ref raft) = self.raft {
            // Get current membership and add the new node
            let current_members = {
                let members = self.members.read().await;
                members.clone()
            };
            
            let mut new_members = current_members;
            new_members.insert(node_id);
            
            // Use Raft's change_membership to add node via consensus
            raft.change_membership(new_members, false).await.map_err(|e| {
                crate::error::ConfluxError::raft(format!("Failed to add node via Raft: {}", e))
            })?;
            
            // Update local members after successful consensus
            {
                let mut members = self.members.write().await;
                members.insert(node_id);
            }
            
            info!("Node {} added to cluster successfully via Raft consensus", node_id);
        } else {
            return Err(crate::error::ConfluxError::raft("Raft not initialized"));
        }
        
        Ok(())
    }

    /// Remove a node from the cluster using Raft consensus
    pub async fn remove_node(&self, node_id: NodeId) -> Result<()> {
        info!("Removing node {} from cluster via Raft consensus", node_id);

        if let Some(ref raft) = self.raft {
            // Get current membership and remove the node
            let current_members = {
                let members = self.members.read().await;
                members.clone()
            };
            
            if current_members.len() <= 1 {
                return Err(crate::error::ConfluxError::raft(
                    "Cannot remove last node from cluster",
                ));
            }
            
            let mut new_members = current_members;
            new_members.remove(&node_id);
            
            // Use Raft's change_membership to remove node via consensus
            raft.change_membership(new_members, false).await.map_err(|e| {
                crate::error::ConfluxError::raft(format!("Failed to remove node via Raft: {}", e))
            })?;
            
            // Update local members after successful consensus
            {
                let mut members = self.members.write().await;
                members.remove(&node_id);
            }
            
            info!("Node {} removed from cluster successfully via Raft consensus", node_id);
        } else {
            return Err(crate::error::ConfluxError::raft("Raft not initialized"));
        }
        
        Ok(())
    }

    /// Check if this node is the leader
    pub async fn is_leader(&self) -> bool {
        if let Some(ref raft) = self.raft {
            // Use actual Raft instance to check leadership
            match raft.ensure_linearizable().await {
                Ok(_) => true,  // Successfully linearizable means we're the leader
                Err(_) => false, // Failed linearizability check means not leader
            }
        } else {
            false
        }
    }

    /// Get current leader ID
    pub async fn get_leader(&self) -> Option<NodeId> {
        if let Some(ref raft) = self.raft {
            // Use actual Raft metrics to get leader ID
            let metrics = raft.metrics().borrow().clone();
            metrics.current_leader
        } else {
            None
        }
    }

    /// Get current Raft metrics
    pub async fn get_metrics(&self) -> Result<RaftMetrics> {
        if let Some(ref raft) = self.raft {
            // Get real metrics from Raft instance
            let raft_metrics = raft.metrics().borrow().clone();
            
            // Extract membership node IDs from the membership config
            let membership: BTreeSet<NodeId> = raft_metrics.membership_config
                .membership()
                .nodes()
                .map(|(id, _)| *id)
                .collect();

            // Update metrics collector with latest data
            self.metrics_collector.update_node_metrics(
                raft_metrics.current_term,
                raft_metrics.last_log_index.unwrap_or(0),
                raft_metrics.last_applied.map(|id| id.index).unwrap_or(0),
                raft_metrics.current_leader,
                self.is_leader().await,
            ).await;

            Ok(RaftMetrics {
                node_id: self.config.node_id,
                current_term: raft_metrics.current_term,
                last_log_index: raft_metrics.last_log_index.unwrap_or(0),
                last_applied: raft_metrics.last_applied.map(|id| id.index).unwrap_or(0),
                leader_id: raft_metrics.current_leader,
                membership,
                is_leader: self.is_leader().await,
            })
        } else {
            Err(crate::error::ConfluxError::raft("Raft not initialized"))
        }
    }

    /// Get comprehensive metrics report
    pub async fn get_comprehensive_metrics(&self) -> Result<crate::raft::metrics::MetricsReport> {
        Ok(self.metrics_collector.get_metrics_report().await)
    }

    /// Get node health status
    pub async fn get_node_health(&self) -> Result<crate::raft::metrics::NodeHealth> {
        Ok(self.metrics_collector.get_node_health().await)
    }

    /// Get resource usage statistics
    pub fn get_resource_stats(&self) -> ResourceStats {
        self.resource_limiter.get_resource_stats()
    }

    /// Wait for leadership
    pub async fn wait_for_leadership(&self, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            if self.is_leader().await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Err(crate::error::ConfluxError::raft("Timeout waiting for leadership"))
    }

    /// Change membership (add/remove nodes) using Raft consensus
    pub async fn change_membership(&self, new_members: BTreeSet<NodeId>) -> Result<()> {
        if !self.is_leader().await {
            return Err(crate::error::ConfluxError::raft("Only leader can change membership"));
        }

        info!("Changing cluster membership to: {:?} via Raft consensus", new_members);

        if let Some(ref raft) = self.raft {
            // Use Raft's change_membership API for consensus-based membership change
            raft.change_membership(new_members.clone(), false).await.map_err(|e| {
                crate::error::ConfluxError::raft(format!("Failed to change membership via Raft: {}", e))
            })?;

            // Update local membership after successful consensus
            {
                let mut members = self.members.write().await;
                *members = new_members;
            }

            info!("Membership change completed via Raft consensus");
        } else {
            return Err(crate::error::ConfluxError::raft("Raft not initialized"));
        }

        Ok(())
    }

    /// Check if this is a single-node cluster
    async fn is_single_node_cluster(&self) -> bool {
        let members = self.members.read().await;
        members.len() == 1 && members.contains(&self.config.node_id)
    }

    /// Update timeout configuration dynamically
    pub async fn update_timeouts(
        &mut self,
        heartbeat_interval: Option<u64>,
        election_timeout_min: Option<u64>,
        election_timeout_max: Option<u64>,
    ) -> Result<()> {
        info!("Updating Raft timeout configuration");

        // Update configuration
        if let Some(interval) = heartbeat_interval {
            self.config.heartbeat_interval = interval;
        }
        if let Some(min_timeout) = election_timeout_min {
            self.config.election_timeout_min = min_timeout;
        }
        if let Some(max_timeout) = election_timeout_max {
            self.config.election_timeout_max = max_timeout;
        }

        // Validate timeout ranges
        if self.config.election_timeout_min >= self.config.election_timeout_max {
            return Err(crate::error::ConfluxError::raft(
                "Election timeout min must be less than max"
            ));
        }

        if self.config.heartbeat_interval >= self.config.election_timeout_min {
            return Err(crate::error::ConfluxError::raft(
                "Heartbeat interval must be less than election timeout min"
            ));
        }

        info!(
            "Timeout configuration updated: heartbeat={}, election_min={}, election_max={}",
            self.config.heartbeat_interval,
            self.config.election_timeout_min, 
            self.config.election_timeout_max
        );

        // Note: For runtime updates to take effect, the Raft instance would need to be restarted
        // This is a limitation of the current openraft implementation
        warn!("Note: Timeout changes require node restart to take effect");

        Ok(())
    }

    /// Get current timeout configuration
    pub fn get_timeout_config(&self) -> (u64, u64, u64) {
        (
            self.config.heartbeat_interval,
            self.config.election_timeout_min,
            self.config.election_timeout_max,
        )
    }

    /// Initialize a single-node cluster
    async fn initialize_cluster(&self) -> Result<()> {
        if let Some(ref raft) = self.raft {
            info!("Initializing single-node cluster for node {}", self.config.node_id);

            let mut members = BTreeSet::new();
            members.insert(self.config.node_id);

            raft.initialize(members).await.map_err(|e| {
                crate::error::ConfluxError::raft(format!("Failed to initialize cluster: {}", e))
            })?;

            info!("Single-node cluster initialized successfully");
        }
        Ok(())
    }
}

/// Helper function to create a basic node configuration
pub fn create_node_config(node_id: NodeId, address: String) -> NodeConfig {
    NodeConfig {
        node_id,
        address,
        raft_config: RaftConfig::default(),
        network_config: NetworkConfig::default(),
        heartbeat_interval: 150,
        election_timeout_min: 300,
        election_timeout_max: 600,
        resource_limits: ResourceLimits::default(),
    }
}

/// Helper function to create a node configuration with custom timeouts
pub fn create_node_config_with_timeouts(
    node_id: NodeId, 
    address: String,
    heartbeat_interval: u64,
    election_timeout_min: u64,
    election_timeout_max: u64,
) -> NodeConfig {
    NodeConfig {
        node_id,
        address,
        raft_config: RaftConfig::default(),
        network_config: NetworkConfig::default(),
        heartbeat_interval,
        election_timeout_min,
        election_timeout_max,
        resource_limits: ResourceLimits::default(),
    }
}

/// Helper function to create a node configuration with custom resource limits
pub fn create_node_config_with_limits(
    node_id: NodeId,
    address: String,
    resource_limits: ResourceLimits,
) -> NodeConfig {
    NodeConfig {
        node_id,
        address,
        raft_config: RaftConfig::default(),
        network_config: NetworkConfig::default(),
        heartbeat_interval: 150,
        election_timeout_min: 300,
        election_timeout_max: 600,
        resource_limits,
    }
}

// Include tests
#[cfg(test)]
#[path = "node_tests.rs"]
mod node_tests;
