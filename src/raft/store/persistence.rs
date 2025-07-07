use crate::error::Result;
use crate::raft::types::*;
use super::constants::*;
use super::types::Store;
use rocksdb::IteratorMode;
use std::collections::BTreeMap;
use tracing::{debug, error, info, warn};

impl Store {
    /// Load all data from disk into memory cache
    pub async fn load_from_disk(&self) -> Result<()> {
        info!("Loading data from disk into memory cache");
        
        // Load configurations
        self.load_configurations().await?;
        
        // Load versions
        self.load_versions().await?;
        
        // Load name index
        self.load_name_index().await?;
        
        // Load metadata
        self.load_metadata().await?;
        
        info!("Successfully loaded all data from disk");
        Ok(())
    }

    /// Load configurations from RocksDB
    async fn load_configurations(&self) -> Result<()> {
        debug!("Loading configurations from RocksDB");
        
        let cf_configs = self.db.cf_handle(CF_CONFIGS).ok_or_else(|| {
            crate::error::ConfluxError::storage("Configurations column family not found")
        })?;

        let mut configurations = self.configurations.write().await;
        let mut count = 0;

        for item in self.db.iterator_cf(cf_configs, IteratorMode::Start) {
            let (key, value) = item.map_err(|e| {
                crate::error::ConfluxError::storage(format!("Failed to read config: {}", e))
            })?;

            let config_key = String::from_utf8(key.to_vec()).map_err(|e| {
                crate::error::ConfluxError::storage(format!("Invalid config key: {}", e))
            })?;

            let config: Config = serde_json::from_slice(&value).map_err(|e| {
                crate::error::ConfluxError::storage(format!("Failed to deserialize config: {}", e))
            })?;

            configurations.insert(config_key, config);
            count += 1;
        }

        debug!("Loaded {} configurations", count);
        Ok(())
    }

    /// Load versions from RocksDB
    async fn load_versions(&self) -> Result<()> {
        debug!("Loading versions from RocksDB");
        
        let cf_versions = self.db.cf_handle(CF_VERSIONS).ok_or_else(|| {
            crate::error::ConfluxError::storage("Versions column family not found")
        })?;

        let mut versions = self.versions.write().await;
        let mut count = 0;

        for item in self.db.iterator_cf(cf_versions, IteratorMode::Start) {
            let (key, value) = item.map_err(|e| {
                crate::error::ConfluxError::storage(format!("Failed to read version: {}", e))
            })?;

            // Parse version key (config_id + version_id)
            if key.len() < 16 {
                warn!("Invalid version key length: {}", key.len());
                continue;
            }

            let config_id = u64::from_be_bytes([
                key[0], key[1], key[2], key[3], key[4], key[5], key[6], key[7],
            ]);
            let version_id = u64::from_be_bytes([
                key[8], key[9], key[10], key[11], key[12], key[13], key[14], key[15],
            ]);

            let version: ConfigVersion = serde_json::from_slice(&value).map_err(|e| {
                crate::error::ConfluxError::storage(format!("Failed to deserialize version: {}", e))
            })?;

            versions
                .entry(config_id)
                .or_insert_with(BTreeMap::new)
                .insert(version_id, version);
            count += 1;
        }

        debug!("Loaded {} versions", count);
        Ok(())
    }

    /// Load name index from RocksDB
    async fn load_name_index(&self) -> Result<()> {
        debug!("Loading name index from RocksDB");
        
        let cf_meta = self.db.cf_handle(CF_META).ok_or_else(|| {
            crate::error::ConfluxError::storage("Meta column family not found")
        })?;

        let mut name_index = self.name_index.write().await;
        let mut count = 0;

        for item in self.db.iterator_cf(cf_meta, IteratorMode::Start) {
            let (key, value) = item.map_err(|e| {
                crate::error::ConfluxError::storage(format!("Failed to read name index: {}", e))
            })?;

            // Only process name index entries (prefix 0x04)
            if key.len() == 0 || key[0] != 0x04 {
                continue;
            }

            let name_key = String::from_utf8(key[1..].to_vec()).map_err(|e| {
                crate::error::ConfluxError::storage(format!("Invalid name index key: {}", e))
            })?;

            let config_id = u64::from_be_bytes([
                value[0], value[1], value[2], value[3], 
                value[4], value[5], value[6], value[7],
            ]);

            name_index.insert(name_key, config_id);
            count += 1;
        }

        debug!("Loaded {} name index entries", count);
        Ok(())
    }

    /// Load metadata from RocksDB
    async fn load_metadata(&self) -> Result<()> {
        debug!("Loading metadata from RocksDB");
        
        let cf_meta = self.db.cf_handle(CF_META).ok_or_else(|| {
            crate::error::ConfluxError::storage("Meta column family not found")
        })?;

        // Load next_config_id (key: 0x01)
        let next_config_id_key = vec![0x01];
        if let Some(value) = self.db.get_cf(cf_meta, &next_config_id_key).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to read next_config_id: {}", e))
        })? {
            if value.len() >= 8 {
                let next_id = u64::from_be_bytes([
                    value[0], value[1], value[2], value[3], 
                    value[4], value[5], value[6], value[7],
                ]);
                *self.next_config_id.write().await = next_id;
                debug!("Loaded next_config_id: {}", next_id);
            }
        }

        Ok(())
    }

    /// Persist a configuration to RocksDB
    pub async fn persist_config(&self, config_key: &str, config: &Config) -> Result<()> {
        debug!("Persisting config: {}", config_key);
        
        let cf_configs = self.db.cf_handle(CF_CONFIGS).ok_or_else(|| {
            crate::error::ConfluxError::storage("Configurations column family not found")
        })?;

        let cf_meta = self.db.cf_handle(CF_META).ok_or_else(|| {
            crate::error::ConfluxError::storage("Meta column family not found")
        })?;

        // Serialize config
        let config_data = serde_json::to_vec(config).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to serialize config: {}", e))
        })?;

        // Store config
        self.db.put_cf(cf_configs, config_key.as_bytes(), config_data).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to store config: {}", e))
        })?;

        // Update name index
        let name_index_key = make_name_index_key(&config.namespace, &config.name);
        let config_id_bytes = config.id.to_be_bytes();
        self.db.put_cf(cf_meta, &name_index_key, &config_id_bytes).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to update name index: {}", e))
        })?;

        debug!("Successfully persisted config: {}", config_key);
        Ok(())
    }

    /// Persist a version to RocksDB
    pub async fn persist_version(&self, version: &ConfigVersion) -> Result<()> {
        debug!("Persisting version: config_id={}, version_id={}", version.config_id, version.id);
        
        let cf_versions = self.db.cf_handle(CF_VERSIONS).ok_or_else(|| {
            crate::error::ConfluxError::storage("Versions column family not found")
        })?;

        // Create version key (config_id + version_id)
        let version_key = make_version_key(version.config_id, version.id);

        // Serialize version
        let version_data = serde_json::to_vec(version).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to serialize version: {}", e))
        })?;

        // Store version
        self.db.put_cf(cf_versions, &version_key, version_data).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to store version: {}", e))
        })?;

        debug!("Successfully persisted version: config_id={}, version_id={}", version.config_id, version.id);
        Ok(())
    }

    /// Persist metadata to RocksDB
    pub async fn persist_metadata(&self) -> Result<()> {
        debug!("Persisting metadata");
        
        let cf_meta = self.db.cf_handle(CF_META).ok_or_else(|| {
            crate::error::ConfluxError::storage("Meta column family not found")
        })?;

        // Persist next_config_id
        let next_config_id_key = vec![0x01];
        let next_id = *self.next_config_id.read().await;
        let next_id_bytes = next_id.to_be_bytes();
        
        self.db.put_cf(cf_meta, &next_config_id_key, &next_id_bytes).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to persist next_config_id: {}", e))
        })?;

        debug!("Successfully persisted metadata");
        Ok(())
    }

    /// Delete a configuration from RocksDB
    pub async fn delete_config_from_disk(&self, config_key: &str, config: &Config) -> Result<()> {
        debug!("Deleting config from disk: {}", config_key);
        
        let cf_configs = self.db.cf_handle(CF_CONFIGS).ok_or_else(|| {
            crate::error::ConfluxError::storage("Configurations column family not found")
        })?;

        let cf_meta = self.db.cf_handle(CF_META).ok_or_else(|| {
            crate::error::ConfluxError::storage("Meta column family not found")
        })?;

        // Delete config
        self.db.delete_cf(cf_configs, config_key.as_bytes()).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to delete config: {}", e))
        })?;

        // Delete name index
        let name_index_key = make_name_index_key(&config.namespace, &config.name);
        self.db.delete_cf(cf_meta, &name_index_key).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to delete name index: {}", e))
        })?;

        debug!("Successfully deleted config from disk: {}", config_key);
        Ok(())
    }

    /// Delete a version from RocksDB
    pub async fn delete_version_from_disk(&self, config_id: u64, version_id: u64) -> Result<()> {
        debug!("Deleting version from disk: config_id={}, version_id={}", config_id, version_id);
        
        let cf_versions = self.db.cf_handle(CF_VERSIONS).ok_or_else(|| {
            crate::error::ConfluxError::storage("Versions column family not found")
        })?;

        // Create version key
        let version_key = make_version_key(config_id, version_id);

        // Delete version
        self.db.delete_cf(cf_versions, &version_key).map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to delete version: {}", e))
        })?;

        debug!("Successfully deleted version from disk: config_id={}, version_id={}", config_id, version_id);
        Ok(())
    }

    /// Force flush all data to disk
    pub async fn flush_to_disk(&self) -> Result<()> {
        debug!("Flushing all data to disk");
        
        self.db.flush().map_err(|e| {
            crate::error::ConfluxError::storage(format!("Failed to flush to disk: {}", e))
        })?;

        debug!("Successfully flushed all data to disk");
        Ok(())
    }

    /// Get database statistics
    pub async fn get_storage_stats(&self) -> Result<StorageStats> {
        debug!("Getting storage statistics");
        
        let configs_count = self.configurations.read().await.len();
        let versions_count = self.versions.read().await.values().map(|v| v.len()).sum();
        let name_index_count = self.name_index.read().await.len();
        let next_config_id = *self.next_config_id.read().await;

        Ok(StorageStats {
            configs_count,
            versions_count,
            name_index_count,
            next_config_id,
        })
    }
}

/// Storage statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageStats {
    pub configs_count: usize,
    pub versions_count: usize,
    pub name_index_count: usize,
    pub next_config_id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn test_load_from_disk() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        // Test loading from empty database
        let result = store.load_from_disk().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_persist_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        // Create test config
        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "app".to_string(),
            env: "dev".to_string(),
        };
        let config = Config {
            id: 1,
            namespace: namespace.clone(),
            name: "test-config".to_string(),
            latest_version_id: 1,
            releases: vec![],
            schema: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let config_key = make_config_key(&namespace, "test-config");

        // Persist config
        let result = store.persist_config(&config_key, &config).await;
        assert!(result.is_ok());

        // Clear memory cache and reload from disk using the same store instance
        {
            let mut configs = store.configurations.write().await;
            configs.clear();
        }
        
        // Reload from disk
        let load_result = store.load_from_disk().await;
        assert!(load_result.is_ok());
        
        // Check if config was loaded
        let loaded_config = store.get_config(&namespace, "test-config").await;
        assert!(loaded_config.is_some());
        assert_eq!(loaded_config.unwrap().id, 1);
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let stats = store.get_storage_stats().await.unwrap();
        assert_eq!(stats.configs_count, 0);
        assert_eq!(stats.versions_count, 0);
        assert_eq!(stats.name_index_count, 0);
        assert_eq!(stats.next_config_id, 1);
    }
}
