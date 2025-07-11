//! 为 openraft 0.9 实现新的 storage v2 接口
//! 
//! 这个文件实现了 RaftLogStorage 和 RaftStateMachine 的分离接口

use crate::raft::types::*;
use super::types::Store;
use openraft::{
    storage::{LogState, Snapshot, SnapshotMeta, RaftLogStorage, RaftStateMachine, LogFlushed},
    Entry, LogId, OptionalSend, RaftSnapshotBuilder, 
    StorageError, StorageIOError, StoredMembership, Vote,
};
use std::sync::Arc;

// 实现 RaftLogStorage for Arc<Store>
impl RaftLogStorage<TypeConfig> for Arc<Store> {
    type LogReader = Arc<Store>;

    async fn get_log_state(&mut self) -> Result<LogState<TypeConfig>, StorageError<NodeId>> {
        let logs = self.logs.read().await;
        let last_serialized = logs.iter().next_back().map(|(_, ent)| ent);

        let last = match last_serialized {
            None => None,
            Some(entry) => {
                let entry: Entry<TypeConfig> = serde_json::from_str(entry)
                    .map_err(|e| StorageIOError::read_logs(&e))?;
                Some(entry.log_id)
            }
        };

        let last_purged = *self.last_purged_log_id.read().await;

        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> Result<(), StorageError<NodeId>> {
        let mut current_vote = self.vote.write().await;
        *current_vote = Some(vote.clone());
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        let current_vote = self.vote.read().await;
        Ok(current_vote.clone())
    }

    async fn append<I>(&mut self, entries: I, callback: LogFlushed<TypeConfig>) -> Result<(), StorageError<NodeId>>
    where
        I: IntoIterator<Item = Entry<TypeConfig>> + OptionalSend,
    {
        let mut logs = self.logs.write().await;
        for entry in entries {
            let log_id = entry.log_id;
            let serialized = serde_json::to_string(&entry)
                .map_err(|e| StorageIOError::write_logs(&e))?;
            logs.insert(log_id.index, serialized);
        }
        
        // 通知日志已写入
        callback.log_io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let mut logs = self.logs.write().await;
        let keys: Vec<_> = logs.range(log_id.index..).map(|(k, _)| *k).collect();
        for key in keys {
            logs.remove(&key);
        }
        Ok(())
    }

    async fn purge(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let mut logs = self.logs.write().await;
        let keys: Vec<_> = logs.range(..=log_id.index).map(|(k, _)| *k).collect();
        for key in keys {
            logs.remove(&key);
        }
        
        let mut last_purged = self.last_purged_log_id.write().await;
        *last_purged = Some(log_id);
        Ok(())
    }
}

// RaftLogReader 实现已经存在于 raft_impl.rs 中，这里不需要重复实现

// 实现 RaftStateMachine for ConfluxStateMachineWrapper
impl RaftStateMachine<TypeConfig> for crate::raft::state_machine::ConfluxStateMachineWrapper {
    async fn applied_state(
        &mut self,
    ) -> Result<
        (Option<LogId<NodeId>>, StoredMembership<NodeId, Node>),
        StorageError<NodeId>,
    > {
        let (last_applied, membership) = self.get_state_info().await;
        Ok((last_applied, membership))
    }

    async fn apply<I>(&mut self, entries: I) -> Result<Vec<ClientWriteResponse>, StorageError<NodeId>>
    where
        I: IntoIterator<Item = Entry<TypeConfig>> + OptionalSend,
    {
        let entries: Vec<_> = entries.into_iter().collect();
        let mut sm = self.inner().write().await;
        sm.apply_entries(&entries).await
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<<TypeConfig as openraft::RaftTypeConfig>::SnapshotData>, StorageError<NodeId>> {
        tracing::debug!("Beginning to receive snapshot");
        Ok(Box::new(std::io::Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<NodeId, Node>,
        snapshot: Box<<TypeConfig as openraft::RaftTypeConfig>::SnapshotData>,
    ) -> Result<(), StorageError<NodeId>> {
        tracing::debug!("Installing snapshot: {:?}", meta);
        
        let data = snapshot.into_inner();
        let mut sm = self.inner().write().await;
        sm.restore_from_snapshot(&data).await?;
        
        tracing::info!("Snapshot installed successfully");
        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<TypeConfig>>, StorageError<NodeId>> {
        tracing::debug!("Getting current snapshot");
        
        let mut builder = crate::raft::state_machine::ConfluxSnapshotBuilder::new(self.inner().clone());
        match builder.build_snapshot().await {
            Ok(snapshot) => Ok(Some(snapshot)),
            Err(e) => {
                tracing::error!("Failed to build snapshot: {}", e);
                Ok(None)
            }
        }
    }

    type SnapshotBuilder = crate::raft::state_machine::ConfluxSnapshotBuilder;

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        crate::raft::state_machine::ConfluxSnapshotBuilder::new(self.inner().clone())
    }
}