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
        // Check if config exists using the new helper method
        let (config_key, existing_config) = match self.find_config_by_id(*config_id).await {
            Ok((key, config)) => (key, config),
            Err(_) => {
                return Ok(Self::create_error_response(format!(
                    "Configuration with ID {} not found",
                    config_id
                )));
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
            let versions = self.versions.read().await;
            let default_format = versions
                .get(config_id)
                .and_then(|config_versions| {
                    config_versions.get(&existing_config.latest_version_id)
                })
                .map(|v| v.format.clone())
                .unwrap_or(ConfigFormat::Json);
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
        if let Err(e) = self.persist_version(&version).await {
            return Ok(Self::create_error_response(format!(
                "Failed to persist version: {}", e
            )));
        }

        {
            let mut configs = self.configurations.write().await;
            if let Some(config) = configs.get_mut(&config_key) {
                config.latest_version_id = version_id;
                config.updated_at = chrono::Utc::now();
                // Persist updated config
                if let Err(e) = self.persist_config(&config_key, config).await {
                    return Ok(Self::create_error_response(format!(
                        "Failed to persist config update: {}", e
                    )));
                }
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

        // Send notification using config info we already have
        let _ = self.change_notifier.send(ConfigChangeEvent {
            config_id: *config_id,
            namespace: existing_config.namespace.clone(),
            name: existing_config.name.clone(),
            version_id,
            change_type: ConfigChangeType::Updated,
        });

        Ok(Self::create_success_response(
            "Configuration version created successfully".to_string(),
            Some(serde_json::json!({
                "config_id": config_id,
                "version_id": version_id
            })),
        ))
    }
}
