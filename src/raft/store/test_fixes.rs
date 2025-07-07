#[cfg(test)]
mod fix_validation_tests {
    use crate::raft::store::Store;
    use crate::raft::types::*;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_release_rules_persistence_fix() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "myapp".to_string(),
            env: "dev".to_string(),
        };

        // Create a config first
        let create_command = RaftCommand::CreateConfig {
            namespace: namespace.clone(),
            name: "test-config.toml".to_string(),
            content: b"key = \"value\"".to_vec(),
            format: ConfigFormat::Toml,
            schema: None,
            creator_id: 1,
            description: "Test config".to_string(),
        };

        let response = store.apply_command(&create_command).await.unwrap();
        assert!(response.success);
        let config_id = response.data.unwrap()["config_id"].as_u64().unwrap();

        // Create a second version
        let version_command = RaftCommand::CreateVersion {
            config_id,
            content: b"key = \"updated_value\"".to_vec(),
            format: Some(ConfigFormat::Toml),
            creator_id: 1,
            description: "Updated config".to_string(),
        };
        let response = store.apply_command(&version_command).await.unwrap();
        assert!(response.success);

        // Update release rules - this should now persist correctly
        let mut labels = BTreeMap::new();
        labels.insert("env".to_string(), "production".to_string());

        let releases = vec![
            Release::new(labels, 2, 10), // Production gets version 2
            Release::default(1),         // Default gets version 1
        ];

        let update_command = RaftCommand::UpdateReleaseRules {
            config_id,
            releases: releases.clone(),
        };

        let response = store.apply_command(&update_command).await.unwrap();
        assert!(response.success, "Release rules update should succeed");

        // Verify the release rules were properly updated and persisted
        let config = store.get_config(&namespace, "test-config.toml").await.unwrap();
        assert_eq!(config.releases.len(), 2);
        assert_eq!(config.releases, releases);

        // Test that creating a new store instance loads the persisted data correctly
        drop(store);
        let new_store = Store::new(temp_dir.path()).await.unwrap();
        let loaded_config = new_store.get_config(&namespace, "test-config.toml").await.unwrap();
        assert_eq!(loaded_config.releases.len(), 2);
        assert_eq!(loaded_config.releases, releases);
    }

    #[tokio::test]
    async fn test_improved_error_handling() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        // Test error handling for non-existent config
        let update_command = RaftCommand::UpdateReleaseRules {
            config_id: 999, // Non-existent config
            releases: vec![Release::default(1)],
        };

        let response = store.apply_command(&update_command).await.unwrap();
        assert!(!response.success);
        assert!(response.message.contains("Configuration with ID 999 not found"));

        // Test error handling for non-existent version
        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "myapp".to_string(),
            env: "dev".to_string(),
        };

        let create_command = RaftCommand::CreateConfig {
            namespace,
            name: "test-config.toml".to_string(),
            content: b"key = \"value\"".to_vec(),
            format: ConfigFormat::Toml,
            schema: None,
            creator_id: 1,
            description: "Test config".to_string(),
        };

        let response = store.apply_command(&create_command).await.unwrap();
        let config_id = response.data.unwrap()["config_id"].as_u64().unwrap();

        // Try to create release rule for non-existent version
        let releases = vec![Release::default(999)]; // Non-existent version

        let update_command = RaftCommand::UpdateReleaseRules {
            config_id,
            releases,
        };

        let response = store.apply_command(&update_command).await.unwrap();
        assert!(!response.success);
        assert!(response.message.contains("Version 999 does not exist"));
    }
}
