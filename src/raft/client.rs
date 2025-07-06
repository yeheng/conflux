use crate::error::Result;
use crate::raft::types::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Client interface for interacting with the Raft cluster
#[derive(Clone)]
pub struct RaftClient {
    /// Local store for direct access (for MVP)
    store: Arc<crate::raft::store::Store>,
    /// Current leader node (for routing requests)
    current_leader: Arc<RwLock<Option<NodeId>>>,
}

/// Client write request wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientWriteRequest {
    /// The command to execute
    pub command: RaftCommand,
    /// Optional request ID for idempotency
    pub request_id: Option<String>,
}

/// Client read request wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientReadRequest {
    /// The type of read operation
    pub operation: ReadOperation,
    /// Optional consistency level
    pub consistency: Option<ReadConsistency>,
}

/// Read operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadOperation {
    /// Get configuration by namespace and name
    GetConfig {
        namespace: ConfigNamespace,
        name: String,
        /// Client labels for release targeting
        client_labels: std::collections::BTreeMap<String, String>,
    },
    /// Get configuration version
    GetConfigVersion { config_id: u64, version_id: u64 },
    /// List configurations in a namespace
    ListConfigs {
        namespace: ConfigNamespace,
        /// Optional prefix filter
        prefix: Option<String>,
    },
}

/// Read consistency levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadConsistency {
    /// Read from any node (eventual consistency)
    Eventual,
    /// Read from leader only (strong consistency)
    Strong,
    /// Read with linearizable semantics
    Linearizable,
}

impl Default for ReadConsistency {
    fn default() -> Self {
        Self::Eventual
    }
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

/// Client read response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientReadResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Response data (if any)
    pub data: Option<serde_json::Value>,
    /// Current leader ID
    pub leader_id: Option<NodeId>,
    /// Consistency level used for this read
    pub consistency_level: ReadConsistency,
}

/// Cluster status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    /// Current leader ID
    pub leader_id: Option<NodeId>,
    /// List of cluster members
    pub members: Vec<NodeId>,
    /// Current term
    pub term: u64,
    /// Last log index
    pub last_log_index: u64,
    /// Commit index
    pub commit_index: u64,
    /// Applied index
    pub applied_index: u64,
}

/// Helper function to create a write request
pub fn create_write_request(command: RaftCommand) -> ClientWriteRequest {
    ClientWriteRequest {
        command,
        request_id: None,
    }
}

/// Helper function to create a read request
pub fn create_read_request(operation: ReadOperation) -> ClientReadRequest {
    ClientReadRequest {
        operation,
        consistency: Some(ReadConsistency::default()),
    }
}

/// Helper function to create a get config request
pub fn create_get_config_request(
    namespace: ConfigNamespace,
    name: String,
    client_labels: std::collections::BTreeMap<String, String>,
) -> ClientReadRequest {
    create_read_request(ReadOperation::GetConfig {
        namespace,
        name,
        client_labels,
    })
}

/// Helper function to create a list configs request
pub fn create_list_configs_request(
    namespace: ConfigNamespace,
    prefix: Option<String>,
) -> ClientReadRequest {
    create_read_request(ReadOperation::ListConfigs { namespace, prefix })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::store::Store;
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn test_client_write() {
        use tempfile::tempdir;
        let temp_dir = tempdir().unwrap();
        let store = Arc::new(Store::new(temp_dir.path()).await.unwrap());
        let client = RaftClient::new(store);

        let command = RaftCommand::CreateConfig {
            namespace: ConfigNamespace {
                tenant: "test".to_string(),
                app: "app".to_string(),
                env: "dev".to_string(),
            },
            name: "test-config".to_string(),
            content: "test content".as_bytes().to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Test configuration".to_string(),
        };

        let request = create_write_request(command);
        let response = client.write(request).await.unwrap();

        assert!(response.success);
    }

    #[tokio::test]
    async fn test_client_read() {
        use tempfile::tempdir;
        let temp_dir = tempdir().unwrap();
        let store = Arc::new(Store::new(temp_dir.path()).await.unwrap());
        let client = RaftClient::new(store);

        let request = create_get_config_request(
            ConfigNamespace {
                tenant: "test".to_string(),
                app: "app".to_string(),
                env: "dev".to_string(),
            },
            "test-config".to_string(),
            BTreeMap::new(),
        );

        let response = client.read(request).await.unwrap();
        assert!(response.success);
    }

    #[tokio::test]
    async fn test_cluster_status() {
        use tempfile::tempdir;
        let temp_dir = tempdir().unwrap();
        let store = Arc::new(Store::new(temp_dir.path()).await.unwrap());
        let client = RaftClient::new(store);

        let status = client.get_cluster_status().await.unwrap();
        assert_eq!(status.members, vec![1]);
    }
}
