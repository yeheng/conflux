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
    BasicNode, Vote,
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
        vote: Vote<NodeId>,
        snapshot: Snapshot<TypeConfig>,
        cancel: impl std::future::Future<Output = ReplicationClosed> + Send + 'static,
        _option: RPCOption,
    ) -> Result<SnapshotResponse<NodeId>, StreamingError<TypeConfig, Fatal<NodeId>>> {
        debug!("Sending full snapshot to node {}", self.target_node_id);

        // Get target node address
        let node_addresses = self.config.node_addresses.read().await;
        let target_address = node_addresses.get(&self.target_node_id)
            .ok_or_else(|| {
                let err = std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("No address found for node {}", self.target_node_id),
                );
                StreamingError::Network(NetworkError::new(&err))
            })?;

        let url = format!("{}/raft/install_snapshot", target_address);

        // Create install snapshot request
        let request = InstallSnapshotRequest {
            vote,
            meta: snapshot.meta.clone(),
            offset: 0,
            data: snapshot.snapshot.into_inner(),
            done: true,
        };

        // Use tokio::select to handle cancellation
        tokio::select! {
            result = self.send_snapshot_with_retry(&url, &request) => {
                match result {
                    Ok(response) => {
                        debug!("Successfully sent snapshot to node {}", self.target_node_id);
                        Ok(SnapshotResponse {
                            vote: response.vote,
                        })
                    }
                    Err(e) => {
                        error!("Failed to send snapshot to node {}: {}", self.target_node_id, e);
                        Err(StreamingError::Network(e))
                    }
                }
            }
            closed = cancel => {
                debug!("Snapshot transmission cancelled for node {}", self.target_node_id);
                Err(StreamingError::Closed(closed))
            }
        }
    }
}

impl ConfluxNetwork {
    /// Send snapshot with retry mechanism for large data
    async fn send_snapshot_with_retry(
        &self,
        url: &str,
        request: &InstallSnapshotRequest<TypeConfig>,
    ) -> Result<InstallSnapshotResponse<NodeId>, NetworkError> {
        let max_attempts = 3;
        let mut delay = Duration::from_millis(500); // Longer delay for snapshots

        for attempt in 1..=max_attempts {
            debug!("Sending snapshot (attempt {}/{})", attempt, max_attempts);

            match self.client
                .post(url)
                .timeout(Duration::from_secs(60)) // Longer timeout for snapshots
                .json(request)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<InstallSnapshotResponse<NodeId>>().await {
                            Ok(data) => {
                                debug!("Snapshot sent successfully");
                                return Ok(data);
                            }
                            Err(e) => {
                                error!("Failed to parse snapshot response (attempt {}/{}): {}", attempt, max_attempts, e);
                                if attempt == max_attempts {
                                    return Err(NetworkError::new(&e));
                                }
                            }
                        }
                    } else {
                        let status = response.status();
                        error!("Snapshot request failed with status {} (attempt {}/{})", status, attempt, max_attempts);
                        if attempt == max_attempts {
                            let err = std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("HTTP error: {}", status),
                            );
                            return Err(NetworkError::new(&err));
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to send snapshot request (attempt {}/{}): {}", attempt, max_attempts, e);
                    if attempt == max_attempts {
                        return Err(NetworkError::new(&e));
                    }
                }
            }

            // Exponential backoff with jitter for snapshots
            let jitter = Duration::from_millis(fastrand::u64(0..=100));
            tokio::time::sleep(delay + jitter).await;
            delay *= 2;
        }

        unreachable!()
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

// Include tests
#[cfg(test)]
#[path = "network_tests.rs"]
mod network_tests;
