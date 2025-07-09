use crate::error::Result;
use super::constants::*;
use super::types::{Store, ConfluxStateMachine};
use rocksdb::{ColumnFamilyDescriptor, Options as RocksDbOptions, DB};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, RwLock};
impl Store {
    /// Create a new Store instance with RocksDB backend
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
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
            state_machine: Arc::new(RwLock::new(ConfluxStateMachine::default())),
            current_snapshot: Arc::new(RwLock::new(None)),
            snapshot_idx: Arc::new(Mutex::new(0)),
        };

        // Load existing data from RocksDB into memory cache
        store.load_from_disk().await?;

        Ok(store)
    }

}
