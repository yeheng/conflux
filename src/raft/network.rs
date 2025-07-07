use crate::raft::types::*;
use openraft::{
    error::{
        Fatal, InstallSnapshotError, NetworkError, RPCError, RaftError, ReplicationClosed,
        StreamingError,
    },
    network::{RPCOption, RaftNetwork, RaftNetworkFactory},
    raft::{
        AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest,
        InstallSnapshotResponse, SnapshotResponse, VoteRequest, VoteResponse,
    },
    storage::Snapshot,
    BasicNode, RPCTypes, Vote,
};
use reqwest::Client;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error};

/// Network configuration for Raft communication
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// HTTP client timeout in seconds
    pub timeout_secs: u64,
    /// Node ID to address mapping
    pub node_addresses: Arc<RwLock<HashMap<NodeId, String>>>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 10,
            node_addresses: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl NetworkConfig {
    /// Create a new network config with node addresses
    pub fn new(node_addresses: HashMap<NodeId, String>) -> Self {
        Self {
            timeout_secs: 10,
            node_addresses: Arc::new(RwLock::new(node_addresses)),
        }
    }

    /// Add a node address
    pub async fn add_node(&self, node_id: NodeId, address: String) {
        self.node_addresses.write().await.insert(node_id, address);
    }

    /// Get node address
    pub async fn get_node_address(&self, node_id: NodeId) -> Option<String> {
        self.node_addresses.read().await.get(&node_id).cloned()
    }
}

/// HTTP-based network implementation for Raft communication
#[derive(Clone)]
pub struct ConfluxNetwork {
    /// Network configuration
    config: NetworkConfig,
    /// HTTP client for making requests
    client: Client,
    /// Target node ID
    target_node_id: NodeId,
}

impl ConfluxNetwork {
    /// Create a new network instance
    pub fn new(config: NetworkConfig, target_node_id: NodeId) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            target_node_id,
        }
    }

    /// Get the target node's address
    async fn get_target_address(&self) -> Result<String, NetworkError> {
        self.config
            .get_node_address(self.target_node_id)
            .await
            .ok_or_else(|| {
                let err = std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Address not found for node {}", self.target_node_id),
                );
                NetworkError::new(&err)
            })
    }

    /// Send request with retry mechanism
    async fn send_with_retry<T, R>(&self, url: &str, request: &T) -> Result<R, NetworkError>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let max_attempts = 3;
        let mut delay = Duration::from_millis(100);

        for attempt in 1..=max_attempts {
            match self.client.post(url).json(request).send().await {
                Ok(response) => match response.json::<R>().await {
                    Ok(data) => return Ok(data),
                    Err(e) => {
                        error!("Failed to parse response (attempt {}/{}): {}", attempt, max_attempts, e);
                        if attempt == max_attempts {
                            return Err(NetworkError::new(&e));
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to send request (attempt {}/{}): {}", attempt, max_attempts, e);
                    if attempt == max_attempts {
                        return Err(NetworkError::new(&e));
                    }
                }
            }

            // Exponential backoff
            tokio::time::sleep(delay).await;
            delay *= 2;
        }

        unreachable!()
    }

    /// Check if target node is reachable
    pub async fn is_reachable(&self) -> bool {
        if let Ok(address) = self.get_target_address().await {
            let url = format!("http://{}/health", address);
            match self.client.get(&url).send().await {
                Ok(response) => response.status().is_success(),
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Get connection statistics
    pub async fn get_connection_stats(&self) -> ConnectionStats {
        ConnectionStats {
            target_node_id: self.target_node_id,
            is_reachable: self.is_reachable().await,
            timeout_secs: self.config.timeout_secs,
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectionStats {
    pub target_node_id: NodeId,
    pub is_reachable: bool,
    pub timeout_secs: u64,
}

impl RaftNetwork<TypeConfig> for ConfluxNetwork {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        debug!("Sending AppendEntries to node {}", self.target_node_id);

        let address = self.get_target_address().await.map_err(RPCError::Network)?;

        let url = format!("http://{}/raft/append_entries", address);

        match self.client.post(&url).json(&rpc).send().await {
            Ok(response) => match response.json::<AppendEntriesResponse<NodeId>>().await {
                Ok(resp) => {
                    debug!(
                        "AppendEntries response received from node {}",
                        self.target_node_id
                    );
                    Ok(resp)
                }
                Err(e) => {
                    error!("Failed to parse AppendEntries response: {}", e);
                    Err(RPCError::Network(NetworkError::new(&e)))
                }
            },
            Err(e) => {
                error!(
                    "Failed to send AppendEntries to node {}: {}",
                    self.target_node_id, e
                );
                Err(RPCError::Network(NetworkError::new(&e)))
            }
        }
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        debug!("Sending Vote to node {}", self.target_node_id);

        let address = self.get_target_address().await.map_err(RPCError::Network)?;

        let url = format!("http://{}/raft/vote", address);

        match self.client.post(&url).json(&rpc).send().await {
            Ok(response) => match response.json::<VoteResponse<NodeId>>().await {
                Ok(resp) => {
                    debug!("Vote response received from node {}", self.target_node_id);
                    Ok(resp)
                }
                Err(e) => {
                    error!("Failed to parse Vote response: {}", e);
                    Err(RPCError::Network(NetworkError::new(&e)))
                }
            },
            Err(e) => {
                error!("Failed to send Vote to node {}: {}", self.target_node_id, e);
                Err(RPCError::Network(NetworkError::new(&e)))
            }
        }
    }

    async fn install_snapshot(
        &mut self,
        rpc: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, Node, RaftError<NodeId, InstallSnapshotError>>,
    > {
        debug!("Sending InstallSnapshot to node {}", self.target_node_id);

        let address = self.get_target_address().await.map_err(RPCError::Network)?;
        let url = format!("http://{}/raft/install_snapshot", address);

        // Send the snapshot installation request
        match self.client.post(&url).json(&rpc).send().await {
            Ok(response) => match response.json::<InstallSnapshotResponse<NodeId>>().await {
                Ok(resp) => {
                    debug!("InstallSnapshot response received from node {}", self.target_node_id);
                    Ok(resp)
                }
                Err(e) => {
                    error!("Failed to parse InstallSnapshot response: {}", e);
                    Err(RPCError::Network(NetworkError::new(&e)))
                }
            },
            Err(e) => {
                error!("Failed to send InstallSnapshot to node {}: {}", self.target_node_id, e);
                Err(RPCError::Network(NetworkError::new(&e)))
            }
        }
    }

    async fn full_snapshot(
        &mut self,
        _vote: Vote<NodeId>,
        _snapshot: Snapshot<TypeConfig>,
        _cancel: impl std::future::Future<Output = ReplicationClosed> + Send + 'static,
        _option: RPCOption,
    ) -> Result<SnapshotResponse<NodeId>, StreamingError<TypeConfig, Fatal<NodeId>>> {
        debug!("Sending full snapshot");
        // For now, return a simple error
        Err(StreamingError::Timeout(openraft::error::Timeout {
            action: RPCTypes::InstallSnapshot,
            target: 0, // dummy target
            id: 0,     // dummy id
            timeout: Duration::from_secs(10),
        }))
    }
}

/// Network factory for creating network instances
#[derive(Clone)]
pub struct ConfluxNetworkFactory {
    config: NetworkConfig,
}

impl ConfluxNetworkFactory {
    pub fn new(config: NetworkConfig) -> Self {
        Self { config }
    }
}

impl RaftNetworkFactory<TypeConfig> for ConfluxNetworkFactory {
    type Network = ConfluxNetwork;

    async fn new_client(&mut self, target: NodeId, _node: &BasicNode) -> Self::Network {
        ConfluxNetwork::new(self.config.clone(), target)
    }
}
