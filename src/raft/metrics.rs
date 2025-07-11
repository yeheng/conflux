use crate::raft::types::NodeId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Comprehensive metrics collection for Raft cluster
#[derive(Debug, Clone)]
pub struct RaftMetricsCollector {
    /// Node-specific metrics
    node_metrics: Arc<RwLock<NodeMetrics>>,
    /// Cluster-wide metrics  
    cluster_metrics: Arc<RwLock<ClusterMetrics>>,
    /// Performance metrics
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Start time for uptime calculation
    start_time: Instant,
}

/// Node-specific metrics
#[derive(Debug, Clone, Default)]
pub struct NodeMetrics {
    /// Node ID
    pub node_id: NodeId,
    /// Current term
    pub current_term: u64,
    /// Last log index
    pub last_log_index: u64,
    /// Last applied index
    pub last_applied: u64,
    /// Leader ID (if known)
    pub leader_id: Option<NodeId>,
    /// Is this node the leader
    pub is_leader: bool,
    /// Number of times this node became leader
    pub leadership_changes: u64,
    /// Total votes received in elections
    pub votes_received: u64,
    /// Total votes granted to other nodes
    pub votes_granted: u64,
    /// Last heartbeat received time
    pub last_heartbeat: Option<Instant>,
    /// Election timeout count
    pub election_timeouts: u64,
    /// Node uptime
    pub uptime: Duration,
}

/// Cluster-wide metrics
#[derive(Debug, Clone, Default)]
pub struct ClusterMetrics {
    /// Total number of nodes in cluster
    pub cluster_size: usize,
    /// Number of healthy nodes
    pub healthy_nodes: usize,
    /// Current membership configuration
    pub membership: HashMap<NodeId, NodeStatus>,
    /// Total number of leadership changes across cluster
    pub total_leadership_changes: u64,
    /// Cluster stability (time since last leadership change)
    pub cluster_stability: Duration,
    /// Last membership change time
    pub last_membership_change: Option<Instant>,
    /// Total membership changes
    pub membership_changes: u64,
}

/// Node status in cluster
#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    /// Node is active and reachable
    Active,
    /// Node is suspected to be down
    Suspected,
    /// Node is confirmed down
    Down,
    /// Node is newly added (not yet confirmed active)
    Joining,
    /// Node is being removed
    Leaving,
}

impl Default for NodeStatus {
    fn default() -> Self {
        NodeStatus::Active
    }
}

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Average request latency (milliseconds)
    pub avg_request_latency: f64,
    /// Request throughput (requests per second)
    pub request_throughput: f64,
    /// Total requests processed
    pub total_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average log replication latency
    pub avg_replication_latency: f64,
    /// Network round-trip times to other nodes
    pub network_rtt: HashMap<NodeId, Duration>,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Disk usage for logs (bytes)
    pub log_storage_usage: u64,
    /// Snapshot size (bytes)
    pub snapshot_size: u64,
    /// Last snapshot creation time
    pub last_snapshot_time: Option<Instant>,
}

impl RaftMetricsCollector {
    /// Create a new metrics collector
    pub fn new(node_id: NodeId) -> Self {
        let node_metrics = NodeMetrics {
            node_id,
            ..Default::default()
        };

        Self {
            node_metrics: Arc::new(RwLock::new(node_metrics)),
            cluster_metrics: Arc::new(RwLock::new(ClusterMetrics::default())),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            start_time: Instant::now(),
        }
    }

    /// Update node metrics
    pub async fn update_node_metrics(
        &self,
        current_term: u64,
        last_log_index: u64,
        last_applied: u64,
        leader_id: Option<NodeId>,
        is_leader: bool,
    ) {
        let mut metrics = self.node_metrics.write().await;
        
        // Check for leadership change
        if metrics.is_leader != is_leader && is_leader {
            metrics.leadership_changes += 1;
            info!(
                "Node {} became leader (total leadership changes: {})",
                metrics.node_id, metrics.leadership_changes
            );
        }

        metrics.current_term = current_term;
        metrics.last_log_index = last_log_index;
        metrics.last_applied = last_applied;
        metrics.leader_id = leader_id;
        metrics.is_leader = is_leader;
        metrics.uptime = self.start_time.elapsed();

        debug!("Updated node metrics for node {}", metrics.node_id);
    }

    /// Record election timeout
    pub async fn record_election_timeout(&self) {
        let mut metrics = self.node_metrics.write().await;
        metrics.election_timeouts += 1;
        warn!(
            "Election timeout recorded for node {} (total: {})",
            metrics.node_id, metrics.election_timeouts
        );
    }

    /// Record vote received
    pub async fn record_vote_received(&self) {
        let mut metrics = self.node_metrics.write().await;
        metrics.votes_received += 1;
        debug!("Vote received by node {}", metrics.node_id);
    }

    /// Record vote granted
    pub async fn record_vote_granted(&self) {
        let mut metrics = self.node_metrics.write().await;
        metrics.votes_granted += 1;
        debug!("Vote granted by node {}", metrics.node_id);
    }

    /// Update heartbeat received time
    pub async fn record_heartbeat(&self) {
        let mut metrics = self.node_metrics.write().await;
        metrics.last_heartbeat = Some(Instant::now());
        debug!("Heartbeat recorded for node {}", metrics.node_id);
    }

    /// Update cluster metrics
    pub async fn update_cluster_metrics(
        &self,
        cluster_size: usize,
        healthy_nodes: usize,
        membership: HashMap<NodeId, NodeStatus>,
    ) {
        let mut metrics = self.cluster_metrics.write().await;
        
        // Check for membership change
        if metrics.membership != membership {
            metrics.membership_changes += 1;
            metrics.last_membership_change = Some(Instant::now());
            info!(
                "Cluster membership changed (total changes: {})",
                metrics.membership_changes
            );
        }

        metrics.cluster_size = cluster_size;
        metrics.healthy_nodes = healthy_nodes;
        metrics.membership = membership;

        debug!("Updated cluster metrics: size={}, healthy={}", cluster_size, healthy_nodes);
    }

    /// Record request metrics
    pub async fn record_request(&self, latency: Duration, success: bool) {
        let mut metrics = self.performance_metrics.write().await;
        metrics.total_requests += 1;
        
        if !success {
            metrics.failed_requests += 1;
        }

        // Update average latency using exponential moving average
        let latency_ms = latency.as_millis() as f64;
        if metrics.avg_request_latency == 0.0 {
            metrics.avg_request_latency = latency_ms;
        } else {
            metrics.avg_request_latency = 0.9 * metrics.avg_request_latency + 0.1 * latency_ms;
        }

        debug!(
            "Request recorded: latency={}ms, success={}, total={}",
            latency_ms, success, metrics.total_requests
        );
    }

    /// Record replication latency
    pub async fn record_replication_latency(&self, latency: Duration) {
        let mut metrics = self.performance_metrics.write().await;
        let latency_ms = latency.as_millis() as f64;
        
        if metrics.avg_replication_latency == 0.0 {
            metrics.avg_replication_latency = latency_ms;
        } else {
            metrics.avg_replication_latency = 0.9 * metrics.avg_replication_latency + 0.1 * latency_ms;
        }

        debug!("Replication latency recorded: {}ms", latency_ms);
    }

    /// Update network RTT to a peer
    pub async fn update_network_rtt(&self, peer_id: NodeId, rtt: Duration) {
        let mut metrics = self.performance_metrics.write().await;
        metrics.network_rtt.insert(peer_id, rtt);
        debug!("Network RTT updated for peer {}: {}ms", peer_id, rtt.as_millis());
    }

    /// Update storage usage metrics
    pub async fn update_storage_metrics(
        &self,
        log_storage_usage: u64,
        snapshot_size: u64,
        memory_usage: u64,
    ) {
        let mut metrics = self.performance_metrics.write().await;
        metrics.log_storage_usage = log_storage_usage;
        metrics.snapshot_size = snapshot_size;
        metrics.memory_usage = memory_usage;

        debug!(
            "Storage metrics updated: log={}KB, snapshot={}KB, memory={}KB",
            log_storage_usage / 1024,
            snapshot_size / 1024,
            memory_usage / 1024
        );
    }

    /// Record snapshot creation
    pub async fn record_snapshot_creation(&self) {
        let mut metrics = self.performance_metrics.write().await;
        metrics.last_snapshot_time = Some(Instant::now());
        info!("Snapshot creation recorded");
    }

    /// Get all metrics as a comprehensive report
    pub async fn get_metrics_report(&self) -> MetricsReport {
        let node_metrics = self.node_metrics.read().await.clone();
        let cluster_metrics = self.cluster_metrics.read().await.clone();
        let performance_metrics = self.performance_metrics.read().await.clone();

        MetricsReport {
            node_metrics,
            cluster_metrics,
            performance_metrics,
            collection_time: Instant::now(),
        }
    }

    /// Calculate current request throughput
    pub async fn calculate_throughput(&self) -> f64 {
        let metrics = self.performance_metrics.read().await;
        let uptime_secs = self.start_time.elapsed().as_secs_f64();
        
        if uptime_secs > 0.0 {
            metrics.total_requests as f64 / uptime_secs
        } else {
            0.0
        }
    }

    /// Get node health status
    pub async fn get_node_health(&self) -> NodeHealth {
        let node_metrics = self.node_metrics.read().await;
        let cluster_metrics = self.cluster_metrics.read().await;
        let performance_metrics = self.performance_metrics.read().await;

        // Determine health based on various factors
        let mut health_score = 100.0;

        // Reduce score based on failed requests
        if performance_metrics.total_requests > 0 {
            let failure_rate = performance_metrics.failed_requests as f64 / performance_metrics.total_requests as f64;
            health_score -= failure_rate * 50.0; // Up to 50 points deduction
        }

        // Reduce score if not receiving heartbeats (when not leader)
        if !node_metrics.is_leader {
            if let Some(last_heartbeat) = node_metrics.last_heartbeat {
                let since_heartbeat = last_heartbeat.elapsed();
                if since_heartbeat > Duration::from_secs(5) {
                    health_score -= 30.0;
                }
            } else {
                health_score -= 40.0; // No heartbeat ever received
            }
        }

        // Reduce score based on cluster health
        if cluster_metrics.cluster_size > 0 {
            let cluster_health_ratio = cluster_metrics.healthy_nodes as f64 / cluster_metrics.cluster_size as f64;
            if cluster_health_ratio < 0.5 {
                health_score -= 20.0; // Cluster majority unhealthy
            }
        }

        let status = if health_score >= 80.0 {
            HealthStatus::Healthy
        } else if health_score >= 50.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };

        NodeHealth {
            status,
            score: health_score.max(0.0).min(100.0),
            last_check: Instant::now(),
        }
    }
}

/// Complete metrics report
#[derive(Debug, Clone)]
pub struct MetricsReport {
    pub node_metrics: NodeMetrics,
    pub cluster_metrics: ClusterMetrics,
    pub performance_metrics: PerformanceMetrics,
    pub collection_time: Instant,
}

/// Node health status
#[derive(Debug, Clone)]
pub struct NodeHealth {
    pub status: HealthStatus,
    pub score: f64, // 0-100
    pub last_check: Instant,
}

/// Health status levels
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}