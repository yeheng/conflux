use crate::error::Result;
use crate::raft::types::*;
use super::super::types::{Store, ConfigChangeEvent, ConfigChangeType};
use std::collections::BTreeMap;

impl Store {
    /// Handle create version command
    pub(crate) async fn handle_create_version(
        &self,
        config_id: &u64,
        content: &[u8],
        format: &Option<ConfigFormat>,
        creator_id: &u64,
        description: &str,
    ) -> Result<ClientWriteResponse> {
        // Check if config exists
        let config_key = {
            let configs = self.configurations.read().await;
            configs
                .iter()
                .find(|(_, config)| config.id == *config_id)
                .map(|(key, _)| key.clone())
        };

        let config_key = match config_key {
            Some(key) => key,
            None => {
                return Ok(ClientWriteResponse {
                    success: false,
                    message: format!("Configuration with ID {} not found", config_id),
                    data: None,
                })
            }
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
            let default_format = configs
                .get(&config_key)
                .and_then(|config| {
                    let versions = self.versions.try_read().ok()?;
                    versions
                        .get(&config.id)?
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
            content.to_vec(),
            version_format,
            *creator_id,
            description.to_string(),
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
            versions
                .entry(*config_id)
                .or_insert_with(BTreeMap::new)
                .insert(version_id, version);
        }

        // Send notification
        let namespace = {
            let configs = self.configurations.read().await;
            configs.get(&config_key).map(|c| c.namespace.clone())
        };

        if let Some(namespace) = namespace {
            let name = config_key
                .split('/')
                .next_back()
                .unwrap_or("unknown")
                .to_string();
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
}
