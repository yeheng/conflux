use crate::error::Result;
use crate::raft::types::*;
use openraft::{
    storage::{LogState, RaftLogReader, RaftSnapshotBuilder, Snapshot, SnapshotMeta},
    Entry, EntryPayload, LogId, OptionalSend, RaftLogId, RaftStorage, StorageError, StorageIOError, StoredMembership, Vote
};
use rocksdb::{DB, Options as RocksDbOptions, ColumnFamilyDescriptor};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io::Cursor;
use std::ops::RangeBounds;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, RwLock};

/// RocksDB column family names
const CF_CONFIGS: &str = "configs";
const CF_VERSIONS: &str = "versions";
const CF_LOGS: &str = "logs";
const CF_META: &str = "meta";

/// Store with RocksDB backend implementing RaftStorage
#[derive(Clone)]
pub struct Store {
    /// RocksDB instance for persistent storage
    db: Arc<DB>,

    /// In-memory cache for configurations
    configurations: Arc<RwLock<BTreeMap<ConfigKey, Config>>>,

    /// In-memory cache for configuration versions
    versions: Arc<RwLock<BTreeMap<u64, BTreeMap<u64, ConfigVersion>>>>,

    /// Name to config ID index
    name_index: Arc<RwLock<BTreeMap<ConfigKey, u64>>>,

    /// Next available config ID
    next_config_id: Arc<RwLock<u64>>,

    /// Change notification broadcaster
    change_notifier: Arc<broadcast::Sender<ConfigChangeEvent>>,

    /// Raft log storage (serialized as JSON strings like memstore)
    logs: Arc<RwLock<BTreeMap<u64, String>>>,

    /// Last purged log ID
    last_purged_log_id: Arc<RwLock<Option<LogId<NodeId>>>>,

    /// Vote storage
    vote: Arc<RwLock<Option<Vote<NodeId>>>>,

    /// State machine data
    state_machine: Arc<RwLock<ConfluxStateMachine>>,

    /// Current snapshot
    current_snapshot: Arc<RwLock<Option<ConfluxSnapshot>>>,

    /// Snapshot index counter
    snapshot_idx: Arc<Mutex<u64>>,
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
#[derive(Debug, Clone)]
pub enum ConfigChangeType {
    Created,
    Updated,
    Deleted,
    ReleaseUpdated,
}

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
        let db = DB::open_cf_descriptors(&opts, path, cfs)
            .map_err(|e| crate::error::ConfluxError::storage(format!("Failed to open RocksDB: {}", e)))?;

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

    /// Load existing data from RocksDB into memory cache
    async fn load_from_disk(&self) -> Result<()> {
        // Load configurations
        let cf_configs = self.db.cf_handle(CF_CONFIGS)
            .ok_or_else(|| crate::error::ConfluxError::storage("Configs column family not found".to_string()))?;

        let mut configs = self.configurations.write().await;
        let mut name_index = self.name_index.write().await;
        let mut max_config_id = 0u64;

        let iter = self.db.iterator_cf(cf_configs, rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, value) = item.map_err(|e| crate::error::ConfluxError::storage(format!("Failed to read config: {}", e)))?;
            let config: Config = serde_json::from_slice(&value)
                .map_err(|e| crate::error::ConfluxError::storage(format!("Failed to deserialize config: {}", e)))?;

            let config_key = String::from_utf8(key.to_vec())
                .map_err(|e| crate::error::ConfluxError::storage(format!("Invalid config key: {}", e)))?;

            max_config_id = max_config_id.max(config.id);
            name_index.insert(config_key.clone(), config.id);
            configs.insert(config_key, config);
        }

        // Update next config ID
        *self.next_config_id.write().await = max_config_id + 1;

        // Load versions
        let cf_versions = self.db.cf_handle(CF_VERSIONS)
            .ok_or_else(|| crate::error::ConfluxError::storage("Versions column family not found".to_string()))?;

        let mut versions = self.versions.write().await;
        let iter = self.db.iterator_cf(cf_versions, rocksdb::IteratorMode::Start);
        for item in iter {
            let (_key, value) = item.map_err(|e| crate::error::ConfluxError::storage(format!("Failed to read version: {}", e)))?;
            let version: ConfigVersion = serde_json::from_slice(&value)
                .map_err(|e| crate::error::ConfluxError::storage(format!("Failed to deserialize version: {}", e)))?;

            versions.entry(version.config_id)
                .or_insert_with(BTreeMap::new)
                .insert(version.id, version);
        }

        Ok(())
    }

    /// Persist configuration to RocksDB
    fn persist_config(&self, config_key: &str, config: &Config) -> Result<()> {
        let cf_configs = self.db.cf_handle(CF_CONFIGS)
            .ok_or_else(|| crate::error::ConfluxError::storage("Configs column family not found".to_string()))?;

        let serialized = serde_json::to_vec(config)
            .map_err(|e| crate::error::ConfluxError::storage(format!("Failed to serialize config: {}", e)))?;

        self.db.put_cf(cf_configs, config_key.as_bytes(), &serialized)
            .map_err(|e| crate::error::ConfluxError::storage(format!("Failed to persist config: {}", e)))?;

        Ok(())
    }

    /// Persist version to RocksDB
    fn persist_version(&self, version: &ConfigVersion) -> Result<()> {
        let cf_versions = self.db.cf_handle(CF_VERSIONS)
            .ok_or_else(|| crate::error::ConfluxError::storage("Versions column family not found".to_string()))?;

        let key = format!("{}:{}", version.config_id, version.id);
        let serialized = serde_json::to_vec(version)
            .map_err(|e| crate::error::ConfluxError::storage(format!("Failed to serialize version: {}", e)))?;

        self.db.put_cf(cf_versions, key.as_bytes(), &serialized)
            .map_err(|e| crate::error::ConfluxError::storage(format!("Failed to persist version: {}", e)))?;

        Ok(())
    }

    /// Subscribe to configuration changes
    pub fn subscribe_changes(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_notifier.subscribe()
    }

    /// Get configuration by namespace and name
    pub async fn get_config(&self, namespace: &ConfigNamespace, name: &str) -> Option<Config> {
        let key = make_config_key(namespace, name);
        self.configurations.read().await.get(&key).cloned()
    }

    /// Get configuration version
    pub async fn get_config_version(&self, config_id: u64, version_id: u64) -> Option<ConfigVersion> {
        self.versions.read().await
            .get(&config_id)?
            .get(&version_id)
            .cloned()
    }

    /// Get published configuration based on client labels
    pub async fn get_published_config(
        &self,
        namespace: &ConfigNamespace,
        name: &str,
        client_labels: &BTreeMap<String, String>,
    ) -> Option<(Config, ConfigVersion)> {
        let config = self.get_config(namespace, name).await?;

        // Find matching release rule using the new method
        let version_id = config.find_matching_release(client_labels)
            .map(|r| r.version_id)
            .or_else(|| config.get_default_release().map(|r| r.version_id))
            .unwrap_or(config.latest_version_id);

        let version = self.get_config_version(config.id, version_id).await?;
        Some((config, version))
    }

    /// Get configuration metadata by ID
    pub async fn get_config_meta(&self, config_id: u64) -> Option<Config> {
        let configs = self.configurations.read().await;
        configs.values().find(|config| config.id == config_id).cloned()
    }

    /// List all versions for a configuration
    pub async fn list_config_versions(&self, config_id: u64) -> Vec<ConfigVersion> {
        let versions = self.versions.read().await;
        versions.get(&config_id)
            .map(|config_versions| config_versions.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get the latest version of a configuration
    pub async fn get_latest_version(&self, config_id: u64) -> Option<ConfigVersion> {
        let config = self.get_config_meta(config_id).await?;
        self.get_config_version(config_id, config.latest_version_id).await
    }

    /// Check if a configuration exists
    pub async fn config_exists(&self, namespace: &ConfigNamespace, name: &str) -> bool {
        self.get_config(namespace, name).await.is_some()
    }

    /// Get all configurations in a namespace
    pub async fn list_configs_in_namespace(&self, namespace: &ConfigNamespace) -> Vec<Config> {
        let configs = self.configurations.read().await;
        configs.values()
            .filter(|config| config.namespace == *namespace)
            .cloned()
            .collect()
    }

    /// Apply a command to the store (for testing)
    pub async fn apply_command(&self, command: &RaftCommand) -> Result<ClientWriteResponse> {
        match command {
            RaftCommand::CreateConfig {
                namespace,
                name,
                content,
                format,
                schema,
                creator_id,
                description,
            } => {
                let config_id = {
                    let mut next_id = self.next_config_id.write().await;
                    let id = *next_id;
                    *next_id += 1;
                    id
                };

                let version_id = 1;
                let now = chrono::Utc::now();

                // Create config
                let config = Config {
                    id: config_id,
                    namespace: namespace.clone(),
                    name: name.clone(),
                    latest_version_id: version_id,
                    releases: vec![Release {
                        labels: BTreeMap::new(), // Default release
                        version_id,
                        priority: 0,
                    }],
                    schema: schema.clone(),
                    created_at: now,
                    updated_at: now,
                };

                // Create version
                let version = ConfigVersion {
                    id: version_id,
                    config_id,
                    content: content.clone(),
                    content_hash: format!("{:x}", sha2::Sha256::digest(content)),
                    format: format.clone(),
                    creator_id: *creator_id,
                    created_at: now,
                    description: description.clone(),
                };

                // Persist to RocksDB and update in-memory state
                let config_name_key = make_config_key(namespace, name);
                self.persist_config(&config_name_key, &config)?;
                self.persist_version(&version)?;

                self.configurations.write().await.insert(config_name_key.clone(), config.clone());
                self.versions.write().await
                    .entry(config_id)
                    .or_insert_with(BTreeMap::new)
                    .insert(version_id, version);
                self.name_index.write().await.insert(config_name_key, config_id);

                // Send notification
                let _ = self.change_notifier.send(ConfigChangeEvent {
                    config_id,
                    namespace: namespace.clone(),
                    name: name.clone(),
                    version_id,
                    change_type: ConfigChangeType::Created,
                });

                Ok(ClientWriteResponse {
                    success: true,
                    message: "Configuration created successfully".to_string(),
                    data: Some(serde_json::json!({
                        "config_id": config_id,
                        "version_id": version_id
                    })),
                })
            }
            RaftCommand::CreateVersion {
                config_id,
                content,
                format,
                creator_id,
                description,
            } => {
                // Check if config exists
                let config_key = {
                    let configs = self.configurations.read().await;
                    configs.iter()
                        .find(|(_, config)| config.id == *config_id)
                        .map(|(key, _)| key.clone())
                };

                let config_key = match config_key {
                    Some(key) => key,
                    None => return Ok(ClientWriteResponse {
                        success: false,
                        message: format!("Configuration with ID {} not found", config_id),
                        data: None,
                    }),
                };

                // Generate new version ID
                let version_id = {
                    let versions = self.versions.read().await;
                    let empty_map = BTreeMap::new();
                    let config_versions = versions.get(config_id).unwrap_or(&empty_map);
                    config_versions.keys().max().copied().unwrap_or(0) + 1
                };

                // Determine format for new version
                let version_format = if let Some(fmt) = format {
                    fmt.clone()
                } else {
                    // Use the format from the config's latest version or default to JSON
                    let configs = self.configurations.read().await;
                    let default_format = configs.get(&config_key)
                        .and_then(|config| {
                            let versions = self.versions.try_read().ok()?;
                            versions.get(&config.id)?
                                .get(&config.latest_version_id)
                                .map(|v| v.format.clone())
                        })
                        .unwrap_or(ConfigFormat::Json);
                    drop(configs);
                    default_format
                };

                // Create new version
                let version = ConfigVersion::new(
                    version_id,
                    *config_id,
                    content.clone(),
                    version_format,
                    *creator_id,
                    description.clone(),
                );

                // Persist version and update config's latest_version_id
                self.persist_version(&version)?;

                {
                    let mut configs = self.configurations.write().await;
                    if let Some(config) = configs.get_mut(&config_key) {
                        config.latest_version_id = version_id;
                        config.updated_at = chrono::Utc::now();
                        // Persist updated config
                        self.persist_config(&config_key, config)?;
                    }
                }

                // Store the new version in memory
                {
                    let mut versions = self.versions.write().await;
                    versions.entry(*config_id)
                        .or_insert_with(BTreeMap::new)
                        .insert(version_id, version);
                }

                // Send notification
                let namespace = {
                    let configs = self.configurations.read().await;
                    configs.get(&config_key).map(|c| c.namespace.clone())
                };

                if let Some(namespace) = namespace {
                    let name = config_key.split('/').last().unwrap_or("unknown").to_string();
                    let _ = self.change_notifier.send(ConfigChangeEvent {
                        config_id: *config_id,
                        namespace,
                        name,
                        version_id,
                        change_type: ConfigChangeType::Updated,
                    });
                }

                Ok(ClientWriteResponse {
                    success: true,
                    message: "Configuration version created successfully".to_string(),
                    data: Some(serde_json::json!({
                        "config_id": config_id,
                        "version_id": version_id
                    })),
                })
            }
            RaftCommand::UpdateReleaseRules {
                config_id,
                releases,
            } => {
                // Find the config by ID
                let config_key = {
                    let configs = self.configurations.read().await;
                    configs.iter()
                        .find(|(_, config)| config.id == *config_id)
                        .map(|(key, _)| key.clone())
                };

                let config_key = match config_key {
                    Some(key) => key,
                    None => return Ok(ClientWriteResponse {
                        success: false,
                        message: format!("Configuration with ID {} not found", config_id),
                        data: None,
                    }),
                };

                // Validate release rules
                for release in releases {
                    // Check if the version exists
                    let version_exists = {
                        let versions = self.versions.read().await;
                        versions.get(config_id)
                            .map(|config_versions| config_versions.contains_key(&release.version_id))
                            .unwrap_or(false)
                    };

                    if !version_exists {
                        return Ok(ClientWriteResponse {
                            success: false,
                            message: format!("Version {} does not exist for config {}", release.version_id, config_id),
                            data: None,
                        });
                    }
                }

                // Update the config's release rules
                {
                    let mut configs = self.configurations.write().await;
                    if let Some(config) = configs.get_mut(&config_key) {
                        config.releases = releases.clone();
                        config.updated_at = chrono::Utc::now();
                    }
                }

                // Send notification
                let (namespace, name) = {
                    let configs = self.configurations.read().await;
                    configs.get(&config_key)
                        .map(|c| (c.namespace.clone(), c.name.clone()))
                        .unwrap_or_else(|| {
                            let parts: Vec<&str> = config_key.split('/').collect();
                            let name = parts.last().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string());
                            (ConfigNamespace {
                                tenant: "unknown".to_string(),
                                app: "unknown".to_string(),
                                env: "unknown".to_string(),
                            }, name)
                        })
                };

                let _ = self.change_notifier.send(ConfigChangeEvent {
                    config_id: *config_id,
                    namespace,
                    name,
                    version_id: 0, // No specific version for release rule updates
                    change_type: ConfigChangeType::ReleaseUpdated,
                });

                Ok(ClientWriteResponse {
                    success: true,
                    message: "Release rules updated successfully".to_string(),
                    data: Some(serde_json::json!({
                        "config_id": config_id,
                        "release_count": releases.len()
                    })),
                })
            }
            RaftCommand::DeleteConfig { config_id } => {
                // Find the config by ID
                let config_key = {
                    let configs = self.configurations.read().await;
                    configs.iter()
                        .find(|(_, config)| config.id == *config_id)
                        .map(|(key, _)| key.clone())
                };

                let config_key = match config_key {
                    Some(key) => key,
                    None => return Ok(ClientWriteResponse {
                        success: false,
                        message: format!("Configuration with ID {} not found", config_id),
                        data: None,
                    }),
                };

                // Get config info for notification
                let (namespace, name) = {
                    let configs = self.configurations.read().await;
                    configs.get(&config_key)
                        .map(|c| (c.namespace.clone(), c.name.clone()))
                        .unwrap_or_else(|| {
                            let parts: Vec<&str> = config_key.split('/').collect();
                            let name = parts.last().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string());
                            (ConfigNamespace {
                                tenant: "unknown".to_string(),
                                app: "unknown".to_string(),
                                env: "unknown".to_string(),
                            }, name)
                        })
                };

                // Remove config and all its versions
                {
                    let mut configs = self.configurations.write().await;
                    configs.remove(&config_key);
                }
                {
                    let mut versions = self.versions.write().await;
                    versions.remove(config_id);
                }
                {
                    let mut name_index = self.name_index.write().await;
                    name_index.remove(&config_key);
                }

                // Send notification
                let _ = self.change_notifier.send(ConfigChangeEvent {
                    config_id: *config_id,
                    namespace,
                    name,
                    version_id: 0,
                    change_type: ConfigChangeType::Deleted,
                });

                Ok(ClientWriteResponse {
                    success: true,
                    message: "Configuration deleted successfully".to_string(),
                    data: Some(serde_json::json!({
                        "config_id": config_id
                    })),
                })
            }
            RaftCommand::DeleteVersions { config_id, version_ids } => {
                // Check if config exists
                let config_exists = {
                    let configs = self.configurations.read().await;
                    configs.iter().any(|(_, config)| config.id == *config_id)
                };

                if !config_exists {
                    return Ok(ClientWriteResponse {
                        success: false,
                        message: format!("Configuration with ID {} not found", config_id),
                        data: None,
                    });
                }

                // Remove specified versions
                let mut deleted_count = 0;
                {
                    let mut versions = self.versions.write().await;
                    if let Some(config_versions) = versions.get_mut(config_id) {
                        for version_id in version_ids {
                            if config_versions.remove(version_id).is_some() {
                                deleted_count += 1;
                            }
                        }
                    }
                }

                Ok(ClientWriteResponse {
                    success: true,
                    message: format!("Deleted {} versions successfully", deleted_count),
                    data: Some(serde_json::json!({
                        "config_id": config_id,
                        "deleted_count": deleted_count
                    })),
                })
            }
        }
    }
}

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
                let entry: Entry<TypeConfig> = serde_json::from_str(serialized)
                    .map_err(|e| StorageIOError::read_logs(&e))?;
                entries.push(entry);
            }
        }
        Ok(entries)
    }
}

// Implement RaftSnapshotBuilder for Arc<Store>
impl RaftSnapshotBuilder<TypeConfig> for Arc<Store> {
    async fn build_snapshot(&mut self) -> std::result::Result<Snapshot<TypeConfig>, StorageError<NodeId>> {
        let data;
        let last_applied_log;
        let last_membership;

        {
            let sm = self.state_machine.read().await;
            data = serde_json::to_vec(&*sm)
                .map_err(|e| StorageIOError::read_state_machine(&e))?;

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

// Implement RaftStorage for Arc<Store>
impl RaftStorage<TypeConfig> for Arc<Store> {
    async fn get_log_state(&mut self) -> std::result::Result<LogState<TypeConfig>, StorageError<NodeId>> {
        let logs = self.logs.read().await;
        let last_serialized = logs.iter().next_back().map(|(_, ent)| ent);

        let last = match last_serialized {
            None => None,
            Some(serialized) => {
                let entry: Entry<TypeConfig> = serde_json::from_str(serialized)
                    .map_err(|e| StorageIOError::read_logs(&e))?;
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

    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> std::result::Result<(), StorageError<NodeId>> {
        let mut h = self.vote.write().await;
        *h = Some(*vote);
        Ok(())
    }

    async fn read_vote(&mut self) -> std::result::Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        Ok(*self.vote.read().await)
    }

    async fn last_applied_state(
        &mut self,
    ) -> std::result::Result<(Option<LogId<NodeId>>, StoredMembership<NodeId, Node>), StorageError<NodeId>> {
        let sm = self.state_machine.read().await;
        Ok((sm.last_applied_log, sm.last_membership.clone()))
    }

    async fn delete_conflict_logs_since(&mut self, log_id: LogId<NodeId>) -> std::result::Result<(), StorageError<NodeId>> {
        let mut logs = self.logs.write().await;
        let keys = logs.range(log_id.index..).map(|(k, _v)| *k).collect::<Vec<_>>();
        for key in keys {
            logs.remove(&key);
        }
        Ok(())
    }

    async fn purge_logs_upto(&mut self, log_id: LogId<NodeId>) -> std::result::Result<(), StorageError<NodeId>> {
        {
            let mut ld = self.last_purged_log_id.write().await;
            assert!(*ld <= Some(log_id));
            *ld = Some(log_id);
        }

        {
            let mut logs = self.logs.write().await;
            let keys = logs.range(..=log_id.index).map(|(k, _v)| *k).collect::<Vec<_>>();
            for key in keys {
                logs.remove(&key);
            }
        }

        Ok(())
    }

    async fn append_to_log<I>(&mut self, entries: I) -> std::result::Result<(), StorageError<NodeId>>
    where
        I: IntoIterator<Item = Entry<TypeConfig>> + OptionalSend
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
                    success: true,
                    message: "Blank entry applied".to_string(),
                    data: None,
                }),
                EntryPayload::Normal(ref data) => {
                    // Apply the command to the configuration store
                    let response = self.apply_command(&data.command).await
                        .unwrap_or_else(|e| ClientWriteResponse {
                            success: false,
                            message: format!("Error applying command: {}", e),
                            data: None,
                        });
                    res.push(response);
                }
                EntryPayload::Membership(ref mem) => {
                    sm.last_membership = StoredMembership::new(Some(entry.log_id), mem.clone());
                    res.push(ClientWriteResponse {
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
    ) -> std::result::Result<Box<<TypeConfig as openraft::RaftTypeConfig>::SnapshotData>, StorageError<NodeId>> {
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
            let new_sm: ConfluxStateMachine = serde_json::from_slice(&new_snapshot.data)
                .map_err(|e| StorageIOError::read_snapshot(Some(new_snapshot.meta.signature()), &e))?;
            let mut sm = self.state_machine.write().await;
            *sm = new_sm;
        }

        // Update current snapshot
        let mut current_snapshot = self.current_snapshot.write().await;
        *current_snapshot = Some(new_snapshot);
        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> std::result::Result<Option<Snapshot<TypeConfig>>, StorageError<NodeId>> {
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

#[cfg(test)]
mod tests;


