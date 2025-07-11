// Module declarations
mod constants;
mod types;
mod store;
mod persistence;
mod config_ops;
mod commands;
mod delete_handlers;
mod raft_impl;
// 注释掉旧的 raft_storage，使用新的 v2 版本
// mod raft_storage;
mod raft_storage_v2;
mod transaction;

// Re-export public types and functions
pub use types::{Store, StateMachineManager};
// Commented out unused exports until needed
// pub use types::{ConfluxStateMachine, ConfluxSnapshot, ConfigChangeEvent, ConfigChangeType};

// Tests module
#[cfg(test)]
mod tests;

#[cfg(test)]
mod raft_storage_tests;

#[cfg(test)]
mod raft_impl_tests;
#[cfg(test)]
mod test_fixes;
