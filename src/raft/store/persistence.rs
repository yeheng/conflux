use crate::error::Result;
use crate::raft::types::*;
use super::constants::*;
use super::types::Store;
use std::collections::BTreeMap;

impl Store {
    /// Load existing data from RocksDB into memory cache
    pub(crate) async fn load_from_disk(&self) -> Result<()> {
        // Load configurations
        let cf_configs = self.db.cf_handle(CF_CONFIGS).ok_or_else(|| {
            crate::error::ConfluxError::storage("Configs column family not found".to_string())
        })?;

        let mut configs = self.configurations.write().await;
        let mut name_index = self.name_index.write().await;
        let mut max_config_id = 0u64;

        let iter = self
            .db
            .iterator_cf(cf_configs, rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, value) = item.map_err(|e| {
                crate::error::ConfluxError::storage(format!("Failed to read config: {}", e))
            })?;
            let config: Config = serde_json::from_slice(&value).map_err(|e| {
                crate::error::ConfluxError::storage(format!("Failed to deserialize config: {}", e))
            })?;

            let config_key = String::from_utf8(key.to_vec()).map_err(|e| {
                crate::error::ConfluxError::storage(format!("Invalid config key: {}", e))
            })?;

            max_config_id = max_config_id.max(config.id);
            name_index.insert(config_key.clone(), config.id);
            configs.insert(config_key, config);
        }

        // Update next config ID
        *self.next_config_id.write().await = max_config_id + 1;

        // Load versions
        let cf_versions = self.db.cf_handle(CF_VERSIONS).ok_or_else(|| {
            crate::error::ConfluxError::storage("Versions column family not found".to_string())
        })?;

        let mut versions = self.versions.write().await;
        let iter = self
            .db
            .iterator_cf(cf_versions, rocksdb::IteratorMode::Start);
        for item in iter {
            let (_key, value) = item.map_err(|e| {
                crate::error::ConfluxError::storage(format!("Failed to read version: {}", e))
            })?;
            let version: ConfigVersion = serde_json::from_slice(&value).map_err(|e| {
                crate::error::ConfluxError::storage(format!("Failed to deserialize version: {}", e))
            })?;

            versions
                .entry(version.config_id)
                .or_insert_with(BTreeMap::new)
                .insert(version.id, version);
        }

        Ok(())
    }

    /// Persist configuration to RocksDB
    pub(crate) fn persist_config(&self, config_key: &str, config: &Config) -> Result<()> {
        let cf_configs = self.db.cf_handle(CF_CONFIGS).ok_or_else(|| {
            crate::error::ConfluxError::storage("Configs column family not found".to_string())
        })?;

        let serialized = serde_json::to_vec(config).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to serialize config: {}", e))
        })?;

        self.db
            .put_cf(cf_configs, config_key.as_bytes(), &serialized)
            .map_err(|e| {
                crate::error::ConfluxError::storage(format!("Failed to persist config: {}", e))
            })?;

        Ok(())
    }

    /// Persist version to RocksDB
    pub(crate) fn persist_version(&self, version: &ConfigVersion) -> Result<()> {
        let cf_versions = self.db.cf_handle(CF_VERSIONS).ok_or_else(|| {
            crate::error::ConfluxError::storage("Versions column family not found".to_string())
        })?;

        let key = format!("{}:{}", version.config_id, version.id);
        let serialized = serde_json::to_vec(version).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to serialize version: {}", e))
        })?;

        self.db
            .put_cf(cf_versions, key.as_bytes(), &serialized)
            .map_err(|e| {
                crate::error::ConfluxError::storage(format!("Failed to persist version: {}", e))
            })?;

        Ok(())
    }
}
