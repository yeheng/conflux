pub mod auth;
pub mod client;
pub mod log_storage;
pub mod metrics;
pub mod network;
pub mod node;
pub mod state_machine;
pub mod store;
pub mod types;
pub mod validation;

// 测试模块
#[cfg(test)]
pub mod cluster_test;
#[cfg(test)]
pub mod simple_cluster_tests;
#[cfg(test)]
pub mod validation_tests;
#[cfg(test)]
pub mod integration_tests;
#[cfg(test)]
pub mod performance_tests;
#[cfg(test)]
pub mod error_handling_tests;


// Commented out unused exports until needed
pub use auth::{RaftAuthzService, AuthorizedRaftOperation};
pub use client::{RaftClient, ClientWriteRequest, ClientReadRequest, ClientReadResponse, ClusterStatus};
pub use log_storage::{ConfluxLogStorage, ConfluxLogReader};
pub use metrics::{RaftMetricsCollector, NodeMetrics, ClusterMetrics, PerformanceMetrics, MetricsReport, NodeHealth, HealthStatus, NodeStatus};
pub use network::{ConfluxNetwork, ConfluxNetworkFactory, NetworkConfig};
pub use node::{create_node_config, create_node_config_with_timeouts, create_node_config_with_limits, NodeConfig, RaftNode, ResourceLimits, ResourceStats};
pub use state_machine::{ConfluxStateMachine, ConfluxStateMachineWrapper, ConfluxSnapshotBuilder};
pub use store::Store;
pub use validation::{RaftInputValidator, ValidationConfig};
