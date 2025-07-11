pub mod client;
pub mod log_storage;
pub mod metrics;
pub mod network;
pub mod node;
pub mod state_machine;
pub mod store;
pub mod types;

// 测试模块
#[cfg(test)]
pub mod cluster_test;
#[cfg(test)]
pub mod simple_cluster_tests;


// Commented out unused exports until needed
pub use client::{RaftClient, ClientWriteRequest, ClientReadRequest, ClientReadResponse, ClusterStatus};
pub use log_storage::{ConfluxLogStorage, ConfluxLogReader};
pub use metrics::{RaftMetricsCollector, NodeMetrics, ClusterMetrics, PerformanceMetrics, MetricsReport, NodeHealth, HealthStatus, NodeStatus};
pub use network::{ConfluxNetwork, ConfluxNetworkFactory, NetworkConfig};
pub use node::{create_node_config, create_node_config_with_timeouts, NodeConfig, RaftNode};
pub use state_machine::{ConfluxStateMachine, ConfluxStateMachineWrapper, ConfluxSnapshotBuilder};
pub use store::Store;
pub use types::*;
