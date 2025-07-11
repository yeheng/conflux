//! 独立的Raft日志存储实现
//! 
//! 这个模块实现了openraft 0.9需要的RaftLogStorage trait，
//! 将日志管理与状态机逻辑分离。

use crate::raft::types::*;
use crate::raft::store::Store;
use openraft::{
    Entry, OptionalSend, RaftLogReader, StorageError,
};
use std::ops::RangeBounds;
use std::sync::Arc;
use tracing::{debug, error};

/// 独立的Raft日志存储实现
/// 
/// 这个实现专注于日志管理，与状态机逻辑完全分离
#[derive(Debug, Clone)]
pub struct ConfluxLogStorage {
    store: Arc<Store>,
}

impl ConfluxLogStorage {
    /// 创建新的日志存储实例
    pub fn new(store: Arc<Store>) -> Self {
        Self { store }
    }
}

impl RaftLogReader<TypeConfig> for ConfluxLogStorage {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + std::fmt::Debug + OptionalSend>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>> {
        let logs = self.store.logs.read().await;
        let mut entries = Vec::new();

        for (index, entry_json) in logs.range(range) {
            match serde_json::from_str::<Entry<TypeConfig>>(entry_json) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    error!("Failed to deserialize log entry at index {}: {}", index, e);
                    return Err(StorageError::IO {
                        source: openraft::StorageIOError::new(
                            openraft::ErrorSubject::Logs,
                            openraft::ErrorVerb::Read,
                            openraft::AnyError::error(format!("Failed to deserialize log entry: {}", e)),
                        ),
                    });
                }
            }
        }
        Ok(entries)
    }
}

/// Raft日志读取器实现
#[derive(Debug, Clone)]
pub struct ConfluxLogReader {
    store: Arc<Store>,
}

impl ConfluxLogReader {
    /// 创建新的日志读取器
    pub fn new(store: Arc<Store>) -> Self {
        Self { store }
    }
}

impl openraft::storage::RaftLogReader<TypeConfig> for ConfluxLogReader {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Send>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>> {
        debug!("Reading log entries in range");
        
        let logs = self.store.logs.read().await;
        let mut entries = Vec::new();
        
        for (index, entry_json) in logs.range(range) {
            match serde_json::from_str::<Entry<TypeConfig>>(entry_json) {
                Ok(entry) => {
                    entries.push(entry);
                }
                Err(e) => {
                    error!("Failed to deserialize log entry at index {}: {}", index, e);
                    return Err(StorageError::IO {
                        source: openraft::StorageIOError::new(
                            openraft::ErrorSubject::Logs,
                            openraft::ErrorVerb::Read,
                            openraft::AnyError::error(format!("Failed to deserialize log entry: {}", e)),
                        ),
                    });
                }
            }
        }
        
        debug!("Retrieved {} log entries", entries.len());
        Ok(entries)
    }
}
