//! Raft输入验证模块
//!
//! 提供Raft集群操作的输入验证功能，包括节点验证、集群验证、超时验证等

mod cluster_validation;
mod comprehensive;
mod config;
mod node_validation;
mod raft_input_validator;
mod timeout_validation;

pub use cluster_validation::ClusterValidator;
pub use comprehensive::{ClusterSuggestions, ComprehensiveValidator};
pub use config::ValidationConfig;
pub use node_validation::NodeValidator;
pub use raft_input_validator::RaftInputValidator;
pub use timeout_validation::TimeoutValidator;

#[cfg(test)]
#[path = "raft_input_validator_test.rs"]
mod raft_input_validator_tests;

#[cfg(test)]
#[path = "timeout_validation_test.rs"]
mod timeouut_validation_tests;

#[cfg(test)]
#[path = "cluster_validation_test.rs"]
mod cluster_validation_test;

#[cfg(test)]
#[path = "comprehensive_test.rs"]
mod comprehensive_test;

#[cfg(test)]
#[path = "config_test.rs"]
mod config_tests;

#[cfg(test)]
#[path = "node_validation_test.rs"]
mod node_validation_tests;