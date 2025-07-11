#[cfg(test)]
mod tests {
    use super::super::types::Store;
    use crate::raft::types::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn create_test_store() -> (Arc<Store>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let (store, _) = Store::new(temp_dir.path()).await.unwrap();
        (Arc::new(store), temp_dir)
    }

    #[tokio::test]
    async fn test_apply_command_create_config() {
        let (store, _temp_dir) = create_test_store().await;
        
        let command = RaftCommand::CreateConfig {
            namespace: ConfigNamespace {
                tenant: "test".to_string(),
                app: "app".to_string(),
                env: "dev".to_string(),
            },
            name: "test-config".to_string(),
            content: b"test content".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Test configuration".to_string(),
        };
        
        let response = store.apply_command(&command).await.unwrap();
        assert!(response.success);
        assert!(response.data.is_some());
        
        // Verify config was created
        let configs = store.list_configs_in_namespace(&ConfigNamespace {
            tenant: "test".to_string(),
            app: "app".to_string(),
            env: "dev".to_string(),
        }).await;
        
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "test-config");
    }

    #[tokio::test]
    async fn test_apply_command_create_version() {
        let (store, _temp_dir) = create_test_store().await;
        
        // First create a config
        let create_config_cmd = RaftCommand::CreateConfig {
            namespace: ConfigNamespace {
                tenant: "test".to_string(),
                app: "app".to_string(),
                env: "dev".to_string(),
            },
            name: "test-config".to_string(),
            content: b"test content".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Test configuration".to_string(),
        };
        
        let config_response = store.apply_command(&create_config_cmd).await.unwrap();
        assert!(config_response.success);

        // Extract config ID from response
        let config_id = config_response.config_id.expect("Config ID should be set in response");
        
        // Now create a version
        let create_version_cmd = RaftCommand::CreateVersion {
            config_id,
            content: serde_json::to_vec(&serde_json::json!({"key": "value"})).unwrap(),
            format: Some(ConfigFormat::Json),
            creator_id: 1,
            description: "Test version".to_string(),
        };
        
        let version_response = store.apply_command(&create_version_cmd).await.unwrap();
        assert!(version_response.success);
        assert!(version_response.data.is_some());
        
        // Verify version was created (should have 2 versions: initial + new one)
        let versions = store.list_config_versions(config_id).await;
        assert_eq!(versions.len(), 2);
        // Find the version we just created
        let test_version = versions.iter().find(|v| v.description == "Test version").unwrap();
        assert_eq!(test_version.description, "Test version");
    }

    // Note: GetConfig command doesn't exist in RaftCommand enum
    // This test is removed as it's not a valid command

    #[tokio::test]
    async fn test_apply_command_update_release_rules() {
        let (store, _temp_dir) = create_test_store().await;
        
        // First create a config
        let create_config_cmd = RaftCommand::CreateConfig {
            namespace: ConfigNamespace {
                tenant: "test".to_string(),
                app: "app".to_string(),
                env: "dev".to_string(),
            },
            name: "test-config".to_string(),
            content: b"test content".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Test configuration".to_string(),
        };
        
        let config_response = store.apply_command(&create_config_cmd).await.unwrap();
        let config_id = config_response.config_id.expect("Config ID should be set in response");
        
        // Update release rules
        let update_rules_cmd = RaftCommand::UpdateReleaseRules {
            config_id,
            releases: vec![
                Release {
                    labels: std::collections::BTreeMap::new(),
                    version_id: 1,
                    priority: 0,
                },
            ],
        };
        
        let rules_response = store.apply_command(&update_rules_cmd).await.unwrap();
        assert!(rules_response.success);
        
        // Verify rules were updated
        let config = store.get_config_meta(config_id).await;
        assert!(config.is_some());
        assert_eq!(config.unwrap().releases.len(), 1);
    }

    #[tokio::test]
    async fn test_apply_command_delete_config() {
        let (store, _temp_dir) = create_test_store().await;
        
        // First create a config
        let create_config_cmd = RaftCommand::CreateConfig {
            namespace: ConfigNamespace {
                tenant: "test".to_string(),
                app: "app".to_string(),
                env: "dev".to_string(),
            },
            name: "test-config".to_string(),
            content: b"test content".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Test configuration".to_string(),
        };
        
        let config_response = store.apply_command(&create_config_cmd).await.unwrap();
        let config_id = config_response.config_id.expect("Config ID should be set in response");

        // Delete the config
        let delete_config_cmd = RaftCommand::DeleteConfig {
            config_id,
        };
        
        let delete_response = store.apply_command(&delete_config_cmd).await.unwrap();
        assert!(delete_response.success);
        
        // Verify config was deleted
        let config = store.get_config_meta(config_id).await;
        assert!(config.is_none());
    }

    #[tokio::test]
    async fn test_apply_command_invalid_config_id() {
        let (store, _temp_dir) = create_test_store().await;

        // Test with invalid config_id
        let delete_invalid_cmd = RaftCommand::DeleteConfig {
            config_id: 99999, // Non-existent config
        };

        let response = store.apply_command(&delete_invalid_cmd).await.unwrap();
        assert!(!response.success);
        assert!(response.message.contains("not found") || !response.message.is_empty());
    }

    #[tokio::test]
    async fn test_apply_command_create_duplicate_config() {
        let (store, _temp_dir) = create_test_store().await;
        
        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "app".to_string(),
            env: "dev".to_string(),
        };
        
        // Create first config
        let create_config_cmd = RaftCommand::CreateConfig {
            namespace: namespace.clone(),
            name: "test-config".to_string(),
            content: b"test content".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Test configuration".to_string(),
        };
        
        let response1 = store.apply_command(&create_config_cmd).await.unwrap();
        assert!(response1.success);
        
        // Try to create duplicate config
        let create_duplicate_cmd = RaftCommand::CreateConfig {
            namespace,
            name: "test-config".to_string(),
            content: b"duplicate content".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Duplicate configuration".to_string(),
        };
        
        let response2 = store.apply_command(&create_duplicate_cmd).await.unwrap();
        assert!(!response2.success);
        assert!(response2.message.contains("already exists") || !response2.message.is_empty());
    }
}
