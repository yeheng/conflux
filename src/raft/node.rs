use crate::config::AppConfig;
use crate::error::Result;
use crate::raft::{
    network::{ConfluxNetworkFactory, NetworkConfig},
    store::Store,
    types::*,
};
use openraft::Config as RaftConfig;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

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
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: 1,
            address: "127.0.0.1:8080".to_string(),
            raft_config: RaftConfig::default(),
            network_config: NetworkConfig::default(),
        }
    }
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
}

impl RaftNode {
    /// Create a new Raft node
    pub async fn new(config: NodeConfig, app_config: &AppConfig) -> Result<Self> {
        info!(
            "Creating Raft node {} at {}",
            config.node_id, config.address
        );

        // Create storage
        let store = Arc::new(Store::new(&app_config.storage.data_dir).await?);

        // Create network factory
        let network_factory = Arc::new(RwLock::new(ConfluxNetworkFactory::new(
            config.network_config.clone(),
        )));

        // Initialize members with self
        let mut members = BTreeSet::new();
        members.insert(config.node_id);

        Ok(Self {
            config,
            store,
            network_factory,
            members: Arc::new(RwLock::new(members)),
            raft: None, // Will be initialized in start()
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

    /// Start the node and initialize Raft instance
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Raft node {}", self.config.node_id);

        // TODO: Properly initialize openraft::Raft instance
        // This requires implementing RaftLogStorage and RaftStateMachine traits for Store
        // The current Store implementation only has RaftStorage which is insufficient
        // 
        // Required steps for full Raft integration:
        // 1. Implement RaftLogStorage trait for Store
        // 2. Implement RaftStateMachine trait for Store  
        // 3. Create proper adaptors using openraft::storage::Adaptor
        // 4. Initialize Raft with: openraft::Raft::new(node_id, config, network, log_store, state_machine)
        //
        // For now, we document this limitation and keep the placeholder implementation

        info!(
            "Raft node {} started (note: full Raft consensus not yet implemented)",
            self.config.node_id
        );
        Ok(())
    }

    /// Get the Raft instance (if available)
    pub fn get_raft(&self) -> Option<&ConfluxRaft> {
        self.raft.as_ref()
    }

    /// Submit a client write request through Raft
    pub async fn client_write(&self, request: ClientRequest) -> Result<ClientWriteResponse> {
        // For MVP, directly apply to store
        // TODO: Route through Raft consensus when properly initialized
        info!(
            "Processing client write through Raft node {}",
            self.config.node_id
        );
        self.store.apply_command(&request.command).await
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

    /// Add a new node to the cluster (placeholder implementation)
    pub async fn add_node(&self, node_id: NodeId, _address: String) -> Result<()> {
        info!("Adding node {} to cluster", node_id);

        // Add to members
        {
            let mut members = self.members.write().await;
            members.insert(node_id);
        }

        info!("Node {} added to cluster successfully", node_id);
        Ok(())
    }

    /// Remove a node from the cluster (placeholder implementation)
    pub async fn remove_node(&self, node_id: NodeId) -> Result<()> {
        info!("Removing node {} from cluster", node_id);

        // Remove from members
        {
            let mut members = self.members.write().await;
            members.remove(&node_id);
        }

        // Check if cluster is empty
        let members = self.members.read().await;
        if members.is_empty() {
            return Err(crate::error::ConfluxError::raft(
                "Cannot remove last node from cluster",
            ));
        }

        info!("Node {} removed from cluster successfully", node_id);
        Ok(())
    }

    /// Check if this node is the leader
    pub async fn is_leader(&self) -> bool {
        if let Some(ref raft) = self.raft {
            // TODO: Use real Raft instance when available
            // raft.is_leader().await
            
            // For now, use simple logic based on node ID
            let members = self.members.read().await;
            members.iter().next() == Some(&self.config.node_id)
        } else {
            false
        }
    }

    /// Get current leader ID
    pub async fn get_leader(&self) -> Option<NodeId> {
        if let Some(ref raft) = self.raft {
            // TODO: Use real Raft instance when available
            // raft.current_leader().await
            
            // For now, return the first node as leader
            let members = self.members.read().await;
            members.iter().next().copied()
        } else {
            None
        }
    }

    /// Get current Raft metrics
    pub async fn get_metrics(&self) -> Result<RaftMetrics> {
        if let Some(ref _raft) = self.raft {
            // TODO: Get real metrics from Raft instance
            // raft.metrics().borrow().clone()
            
            // For now, return placeholder metrics
            Ok(RaftMetrics {
                node_id: self.config.node_id,
                current_term: 1,
                last_log_index: 0,
                last_applied: 0,
                leader_id: self.get_leader().await,
                membership: self.get_members().await,
                is_leader: self.is_leader().await,
            })
        } else {
            Err(crate::error::ConfluxError::raft("Raft not initialized"))
        }
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

    /// Change membership (add/remove nodes)
    pub async fn change_membership(&self, new_members: BTreeSet<NodeId>) -> Result<()> {
        if !self.is_leader().await {
            return Err(crate::error::ConfluxError::raft("Only leader can change membership"));
        }

        info!("Changing cluster membership to: {:?}", new_members);

        // Update local membership
        {
            let mut members = self.members.write().await;
            *members = new_members;
        }

        // TODO: Use real Raft membership change when available
        // if let Some(ref raft) = self.raft {
        //     raft.change_membership(new_members, false).await?;
        // }

        info!("Membership change completed");
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
    }
}
