use crate::error::Result;
use crate::raft::types::*;
use super::types::{Store, ConfigChangeEvent, ConfigChangeType};

impl Store {
    /// Handle delete config command
    pub(crate) async fn handle_delete_config(
        &self,
        config_id: &u64,
    ) -> Result<ClientWriteResponse> {
        // Find the config by ID using the new helper method
        let (config_key, config) = match self.find_config_by_id(*config_id).await {
            Ok((key, config)) => (key, config),
            Err(_) => {
                return Ok(Self::create_error_response(format!(
                    "Configuration with ID {} not found",
                    config_id
                )));
            }
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

        // Send notification using config info we already have
        let _ = self.change_notifier.send(ConfigChangeEvent {
            config_id: *config_id,
            namespace: config.namespace.clone(),
            name: config.name.clone(),
            version_id: 0,
            change_type: ConfigChangeType::Deleted,
        });

        Ok(Self::create_success_response(
            "Configuration deleted successfully".to_string(),
            Some(serde_json::json!({
                "config_id": config_id
            })),
        ))
    }

    /// Handle delete versions command
    pub(crate) async fn handle_delete_versions(
        &self,
        config_id: &u64,
        version_ids: &[u64],
    ) -> Result<ClientWriteResponse> {
        // Check if config exists using the new helper method
        let _config = match self.find_config_by_id(*config_id).await {
            Ok((_, config)) => config,
            Err(_) => {
                return Ok(Self::create_error_response(format!(
                    "Configuration with ID {} not found",
                    config_id
                )));
            }
        };

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

        Ok(Self::create_success_response(
            format!("Deleted {} versions successfully", deleted_count),
            Some(serde_json::json!({
                "config_id": config_id,
                "deleted_count": deleted_count
            })),
        ))
    }
}
