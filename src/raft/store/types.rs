use crate::raft::types::*;
use openraft::{storage::SnapshotMeta, LogId, StoredMembership, Vote};
use rocksdb::DB;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, RwLock};

/// Store with RocksDB backend implementing RaftStorage
#[derive(Clone)]
pub struct Store {
    /// RocksDB instance for persistent storage
    pub(crate) db: Arc<DB>,

    /// In-memory cache for configurations
    pub(crate) configurations: Arc<RwLock<BTreeMap<ConfigKey, Config>>>,

    /// In-memory cache for configuration versions
    pub(crate) versions: Arc<RwLock<BTreeMap<u64, BTreeMap<u64, ConfigVersion>>>>,

    /// Name to config ID index
    pub(crate) name_index: Arc<RwLock<BTreeMap<ConfigKey, u64>>>,

    /// Next available config ID
    pub(crate) next_config_id: Arc<RwLock<u64>>,

    /// Change notification broadcaster
    pub(crate) change_notifier: Arc<broadcast::Sender<ConfigChangeEvent>>,

    /// Raft log storage (serialized as JSON strings like memstore)
    pub(crate) logs: Arc<RwLock<BTreeMap<u64, String>>>,

    /// Last purged log ID
    pub(crate) last_purged_log_id: Arc<RwLock<Option<LogId<NodeId>>>>,

    /// Vote storage
    pub(crate) vote: Arc<RwLock<Option<Vote<NodeId>>>>,

    /// State machine data
    pub(crate) state_machine: Arc<RwLock<ConfluxStateMachine>>,

    /// Current snapshot
    pub(crate) current_snapshot: Arc<RwLock<Option<ConfluxSnapshot>>>,

    /// Snapshot index counter
    pub(crate) snapshot_idx: Arc<Mutex<u64>>,
}

/// State machine for Conflux
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfluxStateMachine {
    pub last_applied_log: Option<LogId<NodeId>>,
    pub last_membership: StoredMembership<NodeId, Node>,
    // Configuration data is stored in the main Store struct
}

/// Snapshot data for Conflux
#[derive(Debug)]
pub struct ConfluxSnapshot {
    pub meta: SnapshotMeta<NodeId, Node>,
    pub data: Vec<u8>,
}

/// Configuration change event
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    pub config_id: u64,
    pub namespace: ConfigNamespace,
    pub name: String,
    pub version_id: u64,
    pub change_type: ConfigChangeType,
}

/// Type of configuration change
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigChangeType {
    Created,
    Updated,
    Deleted,
    ReleaseUpdated,
}
