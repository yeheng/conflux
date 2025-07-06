use crate::error::Result;
use crate::raft::types::*;
use super::types::{Store, ConfigChangeEvent, ConfigChangeType};
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

    /// Handle update release rules command
    pub(crate) async fn handle_update_release_rules(
        &self,
        config_id: &u64,
        releases: &[Release],
    ) -> Result<ClientWriteResponse> {
        // Find the config by ID
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

        // Validate release rules
        for release in releases {
            // Check if the version exists
            let version_exists = {
                let versions = self.versions.read().await;
                versions
                    .get(config_id)
                    .map(|config_versions| {
                        config_versions.contains_key(&release.version_id)
                    })
                    .unwrap_or(false)
            };

            if !version_exists {
                return Ok(ClientWriteResponse {
                    success: false,
                    message: format!(
                        "Version {} does not exist for config {}",
                        release.version_id, config_id
                    ),
                    data: None,
                });
            }
        }

        // Update the config's release rules
        {
            let mut configs = self.configurations.write().await;
            if let Some(config) = configs.get_mut(&config_key) {
                config.releases = releases.to_vec();
                config.updated_at = chrono::Utc::now();
            }
        }

        // Send notification
        let (namespace, name) = {
            let configs = self.configurations.read().await;
            configs
                .get(&config_key)
                .map(|c| (c.namespace.clone(), c.name.clone()))
                .unwrap_or_else(|| {
                    let parts: Vec<&str> = config_key.split('/').collect();
                    let name = parts
                        .last()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    (
                        ConfigNamespace {
                            tenant: "unknown".to_string(),
                            app: "unknown".to_string(),
                            env: "unknown".to_string(),
                        },
                        name,
                    )
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
}
