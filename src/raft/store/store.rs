use crate::error::Result;
use super::constants::*;
use super::types::{Store, StateChangeEvent};
use rocksdb::{ColumnFamilyDescriptor, Options as RocksDbOptions, DB};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, RwLock, mpsc};
impl Store {
    /// Create a new Store instance with RocksDB backend
    /// Returns the store and the event receiver for state machine communication
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<(Self, mpsc::Receiver<StateChangeEvent>)> {
        let (change_notifier, _) = broadcast::channel(1000);

        // Create RocksDB options
        let mut opts = RocksDbOptions::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Define column families
        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_CONFIGS, RocksDbOptions::default()),
            ColumnFamilyDescriptor::new(CF_VERSIONS, RocksDbOptions::default()),
            ColumnFamilyDescriptor::new(CF_LOGS, RocksDbOptions::default()),
            ColumnFamilyDescriptor::new(CF_META, RocksDbOptions::default()),
        ];

        // Open database
        let db = DB::open_cf_descriptors(&opts, path, cfs).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to open RocksDB: {}", e))
        })?;

        // 创建事件通道用于与状态机通信
        let (event_sender, event_receiver) = mpsc::channel(1000);

        let store = Self {
            db: Arc::new(db),
            configurations: Arc::new(RwLock::new(BTreeMap::new())),
            versions: Arc::new(RwLock::new(BTreeMap::new())),
            name_index: Arc::new(RwLock::new(BTreeMap::new())),
            next_config_id: Arc::new(RwLock::new(1)),
            change_notifier: Arc::new(change_notifier),
            logs: Arc::new(RwLock::new(BTreeMap::new())),
            last_purged_log_id: Arc::new(RwLock::new(None)),
            vote: Arc::new(RwLock::new(None)),
            // 移除 state_machine 字段，使用事件通信
            current_snapshot: Arc::new(RwLock::new(None)),
            snapshot_idx: Arc::new(Mutex::new(0)),
            event_sender: Some(event_sender),
        };

        // Load existing data from RocksDB into memory cache
        store.load_from_disk().await?;

        Ok((store, event_receiver))
    }

}
