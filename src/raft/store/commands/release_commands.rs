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

        // Validate release rules - check if all referenced versions exist
        for release in releases {
            if let Err(_) = self.validate_version_exists(*config_id, release.version_id).await {
                return Ok(Self::create_error_response(format!(
                    "Version {} does not exist for config {}",
                    release.version_id, config_id
                )));
            }
        }

        // Update the config's release rules
        {
            let mut configs = self.configurations.write().await;
            if let Some(config) = configs.get_mut(&config_key) {
                config.releases = releases.to_vec();
                config.updated_at = chrono::Utc::now();
                // Persist the updated config to RocksDB
                if let Err(e) = self.persist_config(&config_key, config).await {
                    return Ok(Self::create_error_response(format!(
                        "Failed to persist config update: {}", e
                    )));
                }
            }
        }

        // Send notification using config info we already have
        let _ = self.change_notifier.send(ConfigChangeEvent {
            config_id: *config_id,
            namespace: config.namespace.clone(),
            name: config.name.clone(),
            version_id: 0, // No specific version for release rule updates
            change_type: ConfigChangeType::ReleaseUpdated,
        });

        Ok(Self::create_success_response(
            "Release rules updated successfully".to_string(),
            Some(serde_json::json!({
                "config_id": config_id,
                "release_count": releases.len()
            })),
        ))
    }
}
