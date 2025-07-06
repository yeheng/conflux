use crate::raft::types::*;
use super::types::{Store, ConfluxSnapshot};
use openraft::{
    storage::{RaftLogReader, RaftSnapshotBuilder, Snapshot, SnapshotMeta},
    Entry, OptionalSend, StorageError, StorageIOError,
};
use std::fmt::Debug;
use std::io::Cursor;
use std::ops::RangeBounds;
use std::sync::Arc;

// Implement RaftLogReader for Arc<Store>
impl RaftLogReader<TypeConfig> for Arc<Store> {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug + OptionalSend>(
        &mut self,
        range: RB,
    ) -> std::result::Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>> {
        let mut entries = vec![];
        {
            let logs = self.logs.read().await;
            for (_, serialized) in logs.range(range.clone()) {
                let entry: Entry<TypeConfig> =
                    serde_json::from_str(serialized).map_err(|e| StorageIOError::read_logs(&e))?;
                entries.push(entry);
            }
        }
        Ok(entries)
    }
}

// Implement RaftSnapshotBuilder for Arc<Store>
impl RaftSnapshotBuilder<TypeConfig> for Arc<Store> {
    async fn build_snapshot(
        &mut self,
    ) -> std::result::Result<Snapshot<TypeConfig>, StorageError<NodeId>> {
        let data;
        let last_applied_log;
        let last_membership;

        {
            let sm = self.state_machine.read().await;
            data = serde_json::to_vec(&*sm).map_err(|e| StorageIOError::read_state_machine(&e))?;

            last_applied_log = sm.last_applied_log;
            last_membership = sm.last_membership.clone();
        }

        let snapshot_idx = {
            let mut l = self.snapshot_idx.lock().unwrap();
            *l += 1;
            *l
        };

        let snapshot_id = if let Some(last) = last_applied_log {
            format!("{}-{}-{}", last.leader_id, last.index, snapshot_idx)
        } else {
            format!("--{}", snapshot_idx)
        };

        let meta = SnapshotMeta {
            last_log_id: last_applied_log,
            last_membership,
            snapshot_id,
        };

        let snapshot = ConfluxSnapshot {
            meta: meta.clone(),
            data: data.clone(),
        };

        {
            let mut current_snapshot = self.current_snapshot.write().await;
            *current_snapshot = Some(snapshot);
        }

        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data)),
        })
    }
}
