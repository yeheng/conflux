#[cfg(test)]
mod tests {
    use super::super::types::Store;
    use crate::raft::types::*;
    use openraft::{
        storage::{LogState, Snapshot, SnapshotMeta},
        Entry, EntryPayload, LogId, RaftStorage, StorageError, StoredMembership, Vote,
        CommittedLeaderId, LeaderId,
    };
    use std::io::Cursor;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn create_test_store() -> (Arc<Store>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = Arc::new(Store::new(temp_dir.path()).await.unwrap());
        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_get_log_state_empty() {
        let (mut store, _temp_dir) = create_test_store().await;
        
        let log_state = store.get_log_state().await.unwrap();
        assert_eq!(log_state.last_purged_log_id, None);
        assert_eq!(log_state.last_log_id, None);
    }

    #[tokio::test]
    async fn test_save_and_read_vote() {
        let (mut store, _temp_dir) = create_test_store().await;
        
        // Initially no vote
        let vote = store.read_vote().await.unwrap();
        assert_eq!(vote, None);
        
        // Save a vote
        let test_vote = Vote::new(1, 1);
        store.save_vote(&test_vote).await.unwrap();
        
        // Read it back
        let vote = store.read_vote().await.unwrap();
        assert_eq!(vote, Some(test_vote));
    }

    #[tokio::test]
    async fn test_append_to_log() {
        let (mut store, _temp_dir) = create_test_store().await;
        
        // Create test entries
        let leader_id = CommittedLeaderId::new(1, 1);
        let entries = vec![
            Entry {
                log_id: LogId::new(leader_id, 1),
                payload: EntryPayload::Blank,
            },
            Entry {
                log_id: LogId::new(leader_id, 2),
                payload: EntryPayload::Normal(ClientRequest {
                    command: RaftCommand::CreateConfig {
                        namespace: ConfigNamespace {
                            tenant: "test".to_string(),
                            app: "app".to_string(),
                            env: "dev".to_string(),
                        },
                        name: "test-config".to_string(),
                        content: b"test content".to_vec(),
                        format: ConfigFormat::Json,
                        schema: None,
                        creator_id: 1,
                        description: "Test configuration".to_string(),
                    },
                }),
            },
        ];
        
        // Append entries
        store.append_to_log(entries.clone()).await.unwrap();
        
        // Check log state
        let log_state = store.get_log_state().await.unwrap();
        assert_eq!(log_state.last_log_id, Some(LogId::new(leader_id, 2)));
    }

    #[tokio::test]
    async fn test_delete_conflict_logs_since() {
        let (mut store, _temp_dir) = create_test_store().await;

        // Add some entries
        let leader_id = CommittedLeaderId::new(1, 1);
        let entries = vec![
            Entry {
                log_id: LogId::new(leader_id, 1),
                payload: EntryPayload::Blank,
            },
            Entry {
                log_id: LogId::new(leader_id, 2),
                payload: EntryPayload::Blank,
            },
            Entry {
                log_id: LogId::new(leader_id, 3),
                payload: EntryPayload::Blank,
            },
        ];

        store.append_to_log(entries).await.unwrap();

        // Delete from index 2
        store.delete_conflict_logs_since(LogId::new(leader_id, 2)).await.unwrap();

        // Check that only entry 1 remains
        let log_state = store.get_log_state().await.unwrap();
        assert_eq!(log_state.last_log_id, Some(LogId::new(leader_id, 1)));
    }

    #[tokio::test]
    async fn test_purge_logs_upto() {
        let (mut store, _temp_dir) = create_test_store().await;

        // Add some entries
        let leader_id = CommittedLeaderId::new(1, 1);
        let entries = vec![
            Entry {
                log_id: LogId::new(leader_id, 1),
                payload: EntryPayload::Blank,
            },
            Entry {
                log_id: LogId::new(leader_id, 2),
                payload: EntryPayload::Blank,
            },
            Entry {
                log_id: LogId::new(leader_id, 3),
                payload: EntryPayload::Blank,
            },
        ];

        store.append_to_log(entries).await.unwrap();

        // Purge up to index 2
        store.purge_logs_upto(LogId::new(leader_id, 2)).await.unwrap();

        // Check that only entry 3 remains and purged log id is set
        let log_state = store.get_log_state().await.unwrap();
        assert_eq!(log_state.last_purged_log_id, Some(LogId::new(leader_id, 2)));
        assert_eq!(log_state.last_log_id, Some(LogId::new(leader_id, 3)));
    }

    #[tokio::test]
    async fn test_apply_to_state_machine() {
        let (mut store, _temp_dir) = create_test_store().await;

        // Create test entries with commands
        let leader_id = CommittedLeaderId::new(1, 1);
        let entries = vec![
            Entry {
                log_id: LogId::new(leader_id, 1),
                payload: EntryPayload::Normal(ClientRequest {
                    command: RaftCommand::CreateConfig {
                        namespace: ConfigNamespace {
                            tenant: "test".to_string(),
                            app: "app".to_string(),
                            env: "dev".to_string(),
                        },
                        name: "test-config".to_string(),
                        content: b"test content".to_vec(),
                        format: ConfigFormat::Json,
                        schema: None,
                        creator_id: 1,
                        description: "Test configuration".to_string(),
                    },
                }),
            },
        ];

        // Apply to state machine
        let responses = store.apply_to_state_machine(&entries).await.unwrap();
        assert_eq!(responses.len(), 1);
        assert!(responses[0].success);

        // Check that last applied log is updated
        let (last_applied, _) = store.last_applied_state().await.unwrap();
        assert_eq!(last_applied, Some(LogId::new(leader_id, 1)));
    }

    #[tokio::test]
    async fn test_last_applied_state() {
        let (mut store, _temp_dir) = create_test_store().await;
        
        // Initially no last applied log
        let (last_applied, membership) = store.last_applied_state().await.unwrap();
        assert_eq!(last_applied, None);
        assert_eq!(membership, StoredMembership::default());
    }

    #[tokio::test]
    async fn test_get_log_reader() {
        let (mut store, _temp_dir) = create_test_store().await;
        
        let log_reader = store.get_log_reader().await;
        // Should return a clone of the store
        assert!(Arc::ptr_eq(&store, &log_reader));
    }

    #[tokio::test]
    async fn test_get_snapshot_builder() {
        let (mut store, _temp_dir) = create_test_store().await;
        
        let snapshot_builder = store.get_snapshot_builder().await;
        // Should return a clone of the store
        assert!(Arc::ptr_eq(&store, &snapshot_builder));
    }

    #[tokio::test]
    async fn test_install_snapshot() {
        let (mut store, _temp_dir) = create_test_store().await;
        
        // Create test snapshot data (valid JSON for state machine)
        let snapshot_data = serde_json::to_vec(&serde_json::json!({
            "last_applied_log": null,
            "last_membership": {
                "log_id": null,
                "membership": {
                    "learners": {},
                    "configs": [],
                    "nodes": {}
                }
            }
        })).unwrap();
        let leader_id = CommittedLeaderId::new(1, 1);
        let meta = SnapshotMeta {
            last_log_id: Some(LogId::new(leader_id, 5)),
            last_membership: StoredMembership::default(),
            snapshot_id: "test-snapshot".to_string(),
        };

        // Install snapshot
        store.install_snapshot(&meta, Box::new(Cursor::new(snapshot_data.clone()))).await.unwrap();
        
        // Verify snapshot was installed
        let current_snapshot = store.get_current_snapshot().await.unwrap();
        assert!(current_snapshot.is_some());
        
        let snapshot = current_snapshot.unwrap();
        assert_eq!(snapshot.meta.snapshot_id, "test-snapshot");
        assert_eq!(snapshot.meta.last_log_id, Some(LogId::new(leader_id, 5)));
    }

    #[tokio::test]
    async fn test_get_current_snapshot_none() {
        let (mut store, _temp_dir) = create_test_store().await;
        
        // Initially no snapshot
        let snapshot = store.get_current_snapshot().await.unwrap();
        assert!(snapshot.is_none());
    }
}
