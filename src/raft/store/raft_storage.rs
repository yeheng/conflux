use crate::raft::types::*;
use super::types::{Store, ConfluxStateMachine, ConfluxSnapshot};
use openraft::{
    storage::{LogState, Snapshot, SnapshotMeta},
    Entry, EntryPayload, LogId, OptionalSend, RaftLogId, RaftStorage, StorageError, StorageIOError,
    StoredMembership, Vote,
};
use std::io::Cursor;
use std::sync::Arc;

// Implement RaftStorage for Arc<Store>
impl RaftStorage<TypeConfig> for Arc<Store> {
    async fn get_log_state(
        &mut self,
    ) -> std::result::Result<LogState<TypeConfig>, StorageError<NodeId>> {
        let logs = self.logs.read().await;
        let last_serialized = logs.iter().next_back().map(|(_, ent)| ent);

        let last = match last_serialized {
            None => None,
            Some(serialized) => {
                let entry: Entry<TypeConfig> =
                    serde_json::from_str(serialized).map_err(|e| StorageIOError::read_logs(&e))?;
                Some(*entry.get_log_id())
            }
        };

        let last_purged = *self.last_purged_log_id.read().await;

        let last = match last {
            None => last_purged,
            Some(x) => Some(x),
        };

        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }

    async fn save_vote(
        &mut self,
        vote: &Vote<NodeId>,
    ) -> std::result::Result<(), StorageError<NodeId>> {
        let mut h = self.vote.write().await;
        *h = Some(*vote);
        Ok(())
    }

    async fn read_vote(
        &mut self,
    ) -> std::result::Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        Ok(*self.vote.read().await)
    }

    async fn last_applied_state(
        &mut self,
    ) -> std::result::Result<
        (Option<LogId<NodeId>>, StoredMembership<NodeId, Node>),
        StorageError<NodeId>,
    > {
        let sm = self.state_machine.read().await;
        Ok((sm.last_applied_log, sm.last_membership.clone()))
    }

    async fn delete_conflict_logs_since(
        &mut self,
        log_id: LogId<NodeId>,
    ) -> std::result::Result<(), StorageError<NodeId>> {
        let mut logs = self.logs.write().await;
        let keys = logs
            .range(log_id.index..)
            .map(|(k, _v)| *k)
            .collect::<Vec<_>>();
        for key in keys {
            logs.remove(&key);
        }
        Ok(())
    }

    async fn purge_logs_upto(
        &mut self,
        log_id: LogId<NodeId>,
    ) -> std::result::Result<(), StorageError<NodeId>> {
        {
            let mut ld = self.last_purged_log_id.write().await;
            assert!(*ld <= Some(log_id));
            *ld = Some(log_id);
        }

        {
            let mut logs = self.logs.write().await;
            let keys = logs
                .range(..=log_id.index)
                .map(|(k, _v)| *k)
                .collect::<Vec<_>>();
            for key in keys {
                logs.remove(&key);
            }
        }

        Ok(())
    }

    async fn append_to_log<I>(
        &mut self,
        entries: I,
    ) -> std::result::Result<(), StorageError<NodeId>>
    where
        I: IntoIterator<Item = Entry<TypeConfig>> + OptionalSend,
    {
        let mut logs = self.logs.write().await;
        for entry in entries {
            let s = serde_json::to_string(&entry)
                .map_err(|e| StorageIOError::write_log_entry(*entry.get_log_id(), &e))?;
            logs.insert(entry.log_id.index, s);
        }
        Ok(())
    }

    async fn apply_to_state_machine(
        &mut self,
        entries: &[Entry<TypeConfig>],
    ) -> std::result::Result<Vec<ClientWriteResponse>, StorageError<NodeId>> {
        let mut res = Vec::with_capacity(entries.len());
        let mut sm = self.state_machine.write().await;

        for entry in entries {
            sm.last_applied_log = Some(entry.log_id);

            match &entry.payload {
                EntryPayload::Blank => res.push(ClientWriteResponse {
                    config_id: None,
                    success: true,
                    message: "Blank entry applied".to_string(),
                    data: None,
                }),
                EntryPayload::Normal(ref data) => {
                    // Apply the command to the configuration store using state change method
                    let response = self.apply_state_change(&data.command).await.unwrap_or_else(|e| {
                        ClientWriteResponse {
                            config_id: None,
                            success: false,
                            message: format!("Error applying command: {}", e),
                            data: None,
                        }
                    });
                    res.push(response);
                }
                EntryPayload::Membership(ref mem) => {
                    sm.last_membership = StoredMembership::new(Some(entry.log_id), mem.clone());
                    res.push(ClientWriteResponse {
                        config_id: None,
                        success: true,
                        message: "Membership updated".to_string(),
                        data: None,
                    });
                }
            }
        }
        Ok(res)
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> std::result::Result<
        Box<<TypeConfig as openraft::RaftTypeConfig>::SnapshotData>,
        StorageError<NodeId>,
    > {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<NodeId, Node>,
        snapshot: Box<<TypeConfig as openraft::RaftTypeConfig>::SnapshotData>,
    ) -> std::result::Result<(), StorageError<NodeId>> {
        let new_snapshot = ConfluxSnapshot {
            meta: meta.clone(),
            data: snapshot.into_inner(),
        };

        // Update the state machine
        {
            let new_sm: ConfluxStateMachine =
                serde_json::from_slice(&new_snapshot.data).map_err(|e| {
                    StorageIOError::read_snapshot(Some(new_snapshot.meta.signature()), &e)
                })?;
            let mut sm = self.state_machine.write().await;
            *sm = new_sm;
        }

        // Update current snapshot
        let mut current_snapshot = self.current_snapshot.write().await;
        *current_snapshot = Some(new_snapshot);
        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> std::result::Result<Option<Snapshot<TypeConfig>>, StorageError<NodeId>> {
        match &*self.current_snapshot.read().await {
            Some(snapshot) => {
                let data = snapshot.data.clone();
                Ok(Some(Snapshot {
                    meta: snapshot.meta.clone(),
                    snapshot: Box::new(Cursor::new(data)),
                }))
            }
            None => Ok(None),
        }
    }

    type LogReader = Self;
    type SnapshotBuilder = Self;

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }
}
