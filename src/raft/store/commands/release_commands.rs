use crate::error::Result;
use crate::raft::types::*;
use super::super::types::{Store, ConfigChangeEvent, ConfigChangeType};

impl Store {
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
