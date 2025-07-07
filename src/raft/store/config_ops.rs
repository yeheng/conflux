use crate::error::Result;
use crate::raft::types::*;
use super::types::{Store, ConfigChangeEvent, ConfigChangeType};
use sha2::Digest;
use std::collections::BTreeMap;
use tokio::sync::broadcast;

impl Store {
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
    pub async fn get_config_version(
        &self,
        config_id: u64,
        version_id: u64,
    ) -> Option<ConfigVersion> {
        self.versions
            .read()
            .await
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
        let version_id = config
            .find_matching_release(client_labels)
            .map(|r| r.version_id)
            .or_else(|| config.get_default_release().map(|r| r.version_id))
            .unwrap_or(config.latest_version_id);

        let version = self.get_config_version(config.id, version_id).await?;
        Some((config, version))
    }

    /// Get configuration metadata by ID
    pub async fn get_config_meta(&self, config_id: u64) -> Option<Config> {
        let configs = self.configurations.read().await;
        configs
            .values()
            .find(|config| config.id == config_id)
            .cloned()
    }

    /// List all versions for a configuration
    pub async fn list_config_versions(&self, config_id: u64) -> Vec<ConfigVersion> {
        let versions = self.versions.read().await;
        versions
            .get(&config_id)
            .map(|config_versions| config_versions.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get the latest version of a configuration
    pub async fn get_latest_version(&self, config_id: u64) -> Option<ConfigVersion> {
        let config = self.get_config_meta(config_id).await?;
        self.get_config_version(config_id, config.latest_version_id)
            .await
    }

    /// Check if a configuration exists
    pub async fn config_exists(&self, namespace: &ConfigNamespace, name: &str) -> bool {
        self.get_config(namespace, name).await.is_some()
    }

    /// Get all configurations in a namespace
    pub async fn list_configs_in_namespace(&self, namespace: &ConfigNamespace) -> Vec<Config> {
        let configs = self.configurations.read().await;
        configs
            .values()
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
                self.handle_create_config(
                    namespace, name, content, format, schema, creator_id, description,
                )
                .await
            }
            RaftCommand::CreateVersion {
                config_id,
                content,
                format,
                creator_id,
                description,
            } => {
                self.handle_create_version(config_id, content, format, creator_id, description)
                    .await
            }
            RaftCommand::UpdateReleaseRules {
                config_id,
                releases,
            } => self.handle_update_release_rules(config_id, releases).await,
            RaftCommand::DeleteConfig { config_id } => {
                self.handle_delete_config(config_id).await
            }
            RaftCommand::DeleteVersions {
                config_id,
                version_ids,
            } => self.handle_delete_versions(config_id, version_ids).await,
        }
    }

    /// Handle create config command
    async fn handle_create_config(
        &self,
        namespace: &ConfigNamespace,
        name: &str,
        content: &[u8],
        format: &ConfigFormat,
        schema: &Option<String>,
        creator_id: &u64,
        description: &str,
    ) -> Result<ClientWriteResponse> {
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
            name: name.to_string(),
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
            content: content.to_vec(),
            content_hash: format!("{:x}", sha2::Sha256::digest(content)),
            format: format.clone(),
            creator_id: *creator_id,
            created_at: now,
            description: description.to_string(),
        };

        // Persist to RocksDB and update in-memory state
        let config_name_key = make_config_key(namespace, name);
        self.persist_config(&config_name_key, &config).await?;
        self.persist_version(&version).await?;

        self.configurations
            .write()
            .await
            .insert(config_name_key.clone(), config.clone());
        self.versions
            .write()
            .await
            .entry(config_id)
            .or_insert_with(BTreeMap::new)
            .insert(version_id, version);
        self.name_index
            .write()
            .await
            .insert(config_name_key, config_id);

        // Send notification
        let _ = self.change_notifier.send(ConfigChangeEvent {
            config_id,
            namespace: namespace.clone(),
            name: name.to_string(),
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
}
