use openraft::{BasicNode, Raft};

// 子模块声明
pub mod config;
pub mod version;
pub mod command;
pub mod helpers;

// 重新导出所有公共类型
pub use config::*;
pub use version::*;
pub use command::*;
pub use helpers::*;

/// Node ID type for the Raft cluster
pub type NodeId = u64;

/// Node information for cluster membership
pub type Node = BasicNode;

// Declare Raft types using openraft macro
openraft::declare_raft_types!(
    pub TypeConfig:
        D = ClientRequest,
        R = ClientWriteResponse,
        NodeId = NodeId,
        Node = Node,
        SnapshotData = std::io::Cursor<Vec<u8>>,
);

/// Type alias for the Raft instance
pub type ConfluxRaft = Raft<TypeConfig>;

/// Raft metrics for monitoring and debugging
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RaftMetrics {
    pub node_id: NodeId,
    pub current_term: u64,
    pub last_log_index: u64,
    pub last_applied: u64,
    pub leader_id: Option<NodeId>,
    pub membership: std::collections::BTreeSet<NodeId>,
    pub is_leader: bool,
}
