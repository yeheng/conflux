use crate::raft::types::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
        client_labels: BTreeMap<String, String>,
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
