use crate::error::Result;
use crate::raft::types::*;
use super::types::{Store, ConfigChangeEvent, ConfigChangeType};

impl Store {
    /// Handle delete config command
    pub(crate) async fn handle_delete_config(
        &self,
        config_id: &u64,
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

        // Get config info for notification
        let (namespace, name) = {
            let configs = self.configurations.read().await;
            configs
                .get(&config_key)
                .map(|c| (c.namespace.clone(), c.name.clone()))
                .unwrap_or_else(|| {
                    let parts: Vec<&str> = config_key.split('/').collect();
                    let name = parts
                        .last()
                        .map_or("unknown".to_string(), |s| s.to_string());
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

    /// Handle delete versions command
    pub(crate) async fn handle_delete_versions(
        &self,
        config_id: &u64,
        version_ids: &[u64],
    ) -> Result<ClientWriteResponse> {
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
