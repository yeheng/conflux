use crate::error::Result;
use crate::raft::types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

// 重新导出模块内容
pub mod helpers;
#[cfg(test)]
mod tests;
pub mod types;

pub use types::*;
// pub use helpers::*; // Commented out until needed

/// Client interface for interacting with the Raft cluster
#[derive(Clone)]
pub struct RaftClient {
    /// Local store for direct access (fallback when Raft is not available)
    store: Arc<crate::raft::store::Store>,
    /// Raft node for consensus operations
    raft_node: Option<Arc<RwLock<crate::raft::node::RaftNode>>>,
    /// Current leader node (for routing requests)
    current_leader: Arc<RwLock<Option<NodeId>>>,
}

impl RaftClient {
    /// Create a new Raft client with store only (fallback mode)
    pub fn new(store: Arc<crate::raft::store::Store>) -> Self {
        Self {
            store,
            raft_node: None,
            current_leader: Arc::new(RwLock::new(Some(1))), // Default to node 1 as leader
        }
    }

    /// Create a new Raft client with Raft node (consensus mode)
    pub fn new_with_raft_node(
        store: Arc<crate::raft::store::Store>,
        raft_node: Arc<RwLock<crate::raft::node::RaftNode>>,
    ) -> Self {
        Self {
            store,
            raft_node: Some(raft_node),
            current_leader: Arc::new(RwLock::new(Some(1))), // Default to node 1 as leader
        }
    }

    /// Submit a write request to the cluster
    pub async fn write(&self, request: ClientWriteRequest) -> Result<ClientWriteResponse> {
        info!("Processing client write request: {:?}", request.command);

        // Always use Raft consensus - no fallback to direct store access
        if let Some(ref raft_node) = self.raft_node {
            debug!("Routing write request through Raft consensus");
            let node = raft_node.read().await;

            // Convert ClientWriteRequest to ClientRequest
            let client_request = ClientRequest {
                command: request.command.clone(),
            };

            match node.client_write(client_request).await {
                Ok(response) => {
                    debug!("Raft write completed successfully");
                    return Ok(response);
                }
                Err(e) => {
                    error!("Raft write failed: {}", e);
                    return Err(e);
                }
            }
        }

        // Return error if no Raft node available instead of fallback
        Err(crate::error::ConfluxError::raft(
            "No Raft node available - cannot process write requests",
        ))
    }

    /// Submit a write request with automatic leader detection
    pub async fn write_with_leader_detection(
        &self,
        request: ClientWriteRequest,
    ) -> Result<ClientWriteResponse> {
        // Check if we know the current leader
        let leader_id = self.current_leader.read().await;

        if leader_id.is_none() {
            return Err(crate::error::ConfluxError::raft("No leader available"));
        }

        // For now, just use the local store (same as write method)
        // TODO: Implement actual leader detection and request forwarding
        self.write(request).await
    }

    /// Batch write multiple requests
    pub async fn batch_write(
        &self,
        requests: Vec<ClientWriteRequest>,
    ) -> Result<Vec<ClientWriteResponse>> {
        info!("Processing batch write with {} requests", requests.len());

        let mut responses = Vec::with_capacity(requests.len());

        for request in requests {
            match self.write(request).await {
                Ok(response) => responses.push(response),
                Err(e) => {
                    // For batch operations, we could either fail fast or continue
                    // For now, we fail fast
                    return Err(e);
                }
            }
        }

        debug!("Batch write completed successfully");
        Ok(responses)
    }

    /// Submit a read request to the cluster with linearizability through Raft
    pub async fn read(&self, request: ClientReadRequest) -> Result<ClientReadResponse> {
        debug!("Processing client read request: {:?}", request.operation);

        // Ensure linearizable reads through Raft consensus
        if let Some(ref raft_node) = self.raft_node {
            let node = raft_node.read().await;

            // Ensure we can provide linearizable reads (only leaders can guarantee this)
            if let Some(ref raft) = node.get_raft() {
                // Use ensure_linearizable to make sure we can provide consistent reads
                match raft.ensure_linearizable().await {
                    Ok(_) => {
                        // We're the leader or can provide linearizable reads
                        debug!("Linearizable read confirmed, proceeding with read operation");
                    }
                    Err(e) => {
                        return Err(crate::error::ConfluxError::raft(format!(
                            "Cannot provide linearizable read: {}",
                            e
                        )));
                    }
                }
            } else {
                return Err(crate::error::ConfluxError::raft(
                    "Raft instance not available",
                ));
            }
        } else {
            return Err(crate::error::ConfluxError::raft(
                "No Raft node available for reads",
            ));
        }

        // Now perform the actual read operation
        let data = match request.operation {
            ReadOperation::GetConfig {
                namespace,
                name,
                client_labels,
            } => {
                let result = self
                    .store
                    .get_published_config(&namespace, &name, &client_labels)
                    .await;
                result.map(|(config, version)| {
                    serde_json::json!({
                        "config": config,
                        "version": version
                    })
                })
            }
            ReadOperation::GetConfigVersion {
                config_id,
                version_id,
            } => {
                let result = self.store.get_config_version(config_id, version_id).await;
                result.map(|version| serde_json::json!(version))
            }
            ReadOperation::ListConfigs { namespace, prefix } => {
                // For MVP, return empty list
                // In a real implementation, this would query the store
                let _ = (namespace, prefix);
                Some(serde_json::json!([]))
            }
        };

        let response = ClientReadResponse {
            success: true,
            data,
            leader_id: *self.current_leader.read().await,
            consistency_level: request.consistency.unwrap_or_default(),
        };

        debug!("Client read completed successfully");
        Ok(response)
    }

    /// Get current cluster status
    pub async fn get_cluster_status(&self) -> Result<ClusterStatus> {
        debug!("Getting cluster status");

        let status = ClusterStatus {
            leader_id: *self.current_leader.read().await,
            members: vec![1], // For MVP, single node cluster
            term: 1,
            last_log_index: 0,
            commit_index: 0,
            applied_index: 0,
        };

        Ok(status)
    }

    /// Set the current leader (for testing and manual control)
    pub async fn set_leader(&self, leader_id: Option<NodeId>) {
        let mut current_leader = self.current_leader.write().await;
        *current_leader = leader_id;
        info!("Leader set to: {:?}", leader_id);
    }

    /// Check if the client is connected to the leader
    pub async fn is_connected_to_leader(&self) -> bool {
        self.current_leader.read().await.is_some()
    }

    /// Wait for the cluster to have a leader
    pub async fn wait_for_leader(&self, timeout: std::time::Duration) -> Result<NodeId> {
        let start = std::time::Instant::now();

        loop {
            if let Some(leader_id) = *self.current_leader.read().await {
                return Ok(leader_id);
            }

            if start.elapsed() > timeout {
                return Err(crate::error::ConfluxError::raft(
                    "Timeout waiting for leader",
                ));
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}
