use crate::error::Result;
use crate::raft::types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

// 重新导出模块内容
pub mod types;
pub mod helpers;
#[cfg(test)]
mod tests;

pub use types::*;
pub use helpers::*;

/// Client interface for interacting with the Raft cluster
#[derive(Clone)]
pub struct RaftClient {
    /// Local store for direct access (for MVP)
    store: Arc<crate::raft::store::Store>,
    /// Current leader node (for routing requests)
    current_leader: Arc<RwLock<Option<NodeId>>>,
}

impl RaftClient {
    /// Create a new Raft client
    pub fn new(store: Arc<crate::raft::store::Store>) -> Self {
        Self {
            store,
            current_leader: Arc::new(RwLock::new(Some(1))), // Default to node 1 as leader
        }
    }

    /// Submit a write request to the cluster
    pub async fn write(&self, request: ClientWriteRequest) -> Result<ClientWriteResponse> {
        info!("Processing client write request: {:?}", request.command);

        // For MVP, directly apply to local store
        // In a real implementation, this would route to the leader
        let response = self.store.apply_command(&request.command).await?;

        debug!("Client write completed successfully");
        Ok(response)
    }

    /// Submit a read request to the cluster
    pub async fn read(&self, request: ClientReadRequest) -> Result<ClientReadResponse> {
        debug!("Processing client read request: {:?}", request.operation);

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
