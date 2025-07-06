use crate::raft::types::*;
use openraft::{
    error::{InstallSnapshotError, NetworkError, RPCError, RaftError, ReplicationClosed, StreamingError, Fatal},
    network::{RPCOption, RaftNetwork, RaftNetworkFactory},
    raft::{
        AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
        VoteRequest, VoteResponse, SnapshotResponse,
    },
    storage::Snapshot,
    BasicNode, Vote, RPCTypes,
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
        self.config.get_node_address(self.target_node_id).await
            .ok_or_else(|| {
                let err = std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Address not found for node {}", self.target_node_id)
                );
                NetworkError::new(&err)
            })
    }
}

impl RaftNetwork<TypeConfig> for ConfluxNetwork {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        debug!("Sending AppendEntries to node {}", self.target_node_id);

        let address = self.get_target_address().await
            .map_err(|e| RPCError::Network(e))?;

        let url = format!("http://{}/raft/append_entries", address);

        match self.client.post(&url).json(&rpc).send().await {
            Ok(response) => {
                match response.json::<AppendEntriesResponse<NodeId>>().await {
                    Ok(resp) => {
                        debug!("AppendEntries response received from node {}", self.target_node_id);
                        Ok(resp)
                    }
                    Err(e) => {
                        error!("Failed to parse AppendEntries response: {}", e);
                        Err(RPCError::Network(NetworkError::new(&e)))
                    }
                }
            }
            Err(e) => {
                error!("Failed to send AppendEntries to node {}: {}", self.target_node_id, e);
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

        let address = self.get_target_address().await
            .map_err(|e| RPCError::Network(e))?;

        let url = format!("http://{}/raft/vote", address);

        match self.client.post(&url).json(&rpc).send().await {
            Ok(response) => {
                match response.json::<VoteResponse<NodeId>>().await {
                    Ok(resp) => {
                        debug!("Vote response received from node {}", self.target_node_id);
                        Ok(resp)
                    }
                    Err(e) => {
                        error!("Failed to parse Vote response: {}", e);
                        Err(RPCError::Network(NetworkError::new(&e)))
                    }
                }
            }
            Err(e) => {
                error!("Failed to send Vote to node {}: {}", self.target_node_id, e);
                Err(RPCError::Network(NetworkError::new(&e)))
            }
        }
    }

    async fn install_snapshot(
        &mut self,
        _rpc: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<InstallSnapshotResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId, InstallSnapshotError>>> {
        debug!("Sending InstallSnapshot");
        // For now, return a simple error since we don't have target info
        let error = std::io::Error::new(std::io::ErrorKind::NotConnected, "Network not implemented yet");
        Err(RPCError::Network(NetworkError::new(&error)))
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
            id: 0, // dummy id
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
