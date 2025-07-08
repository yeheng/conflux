#[cfg(test)]
mod tests {
    use crate::raft::{store::types::ConfigChangeType, types::*, Store};
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_exists_false() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let namespace = ConfigNamespace {
            tenant: "nonexistent".to_string(),
            app: "app".to_string(),
            env: "test".to_string(),
        };

        assert!(!store.config_exists(&namespace, "missing.json").await);
    }

    #[tokio::test]
    async fn test_get_config_none() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "app".to_string(),
            env: "prod".to_string(),
        };

        assert!(store.get_config(&namespace, "missing.json").await.is_none());
    }

    #[tokio::test]
    async fn test_get_config_version_none() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        assert!(store.get_config_version(999, 1).await.is_none());
    }

    #[tokio::test]
    async fn test_get_published_config_none() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "app".to_string(),
            env: "dev".to_string(),
        };

        let labels = BTreeMap::new();
        assert!(store.get_published_config(&namespace, "missing.json", &labels).await.is_none());
    }

    #[tokio::test]
    async fn test_get_config_meta_none() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        assert!(store.get_config_meta(999).await.is_none());
    }

    #[tokio::test]
    async fn test_list_config_versions_empty() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let versions = store.list_config_versions(999).await;
        assert!(versions.is_empty());
    }

    #[tokio::test]
    async fn test_get_latest_version_none() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        assert!(store.get_latest_version(999).await.is_none());
    }

    #[tokio::test]
    async fn test_list_configs_in_namespace_empty() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let namespace = ConfigNamespace {
            tenant: "empty".to_string(),
            app: "app".to_string(),
            env: "test".to_string(),
        };

        let configs = store.list_configs_in_namespace(&namespace).await;
        assert!(configs.is_empty());
    }

    #[tokio::test]
    async fn test_create_duplicate_config() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "dup".to_string(),
            env: "test".to_string(),
        };

        // Create first config
        let command1 = RaftCommand::CreateConfig {
            namespace: namespace.clone(),
            name: "duplicate.json".to_string(),
            content: b"{}".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "First config".to_string(),
        };

        let response1 = store.apply_command(&command1).await.unwrap();
        assert!(response1.success);

        // Try to create duplicate
        let command2 = RaftCommand::CreateConfig {
            namespace: namespace.clone(),
            name: "duplicate.json".to_string(),
            content: b"{}".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 2,
            description: "Duplicate config".to_string(),
        };

        let response2 = store.apply_command(&command2).await.unwrap();
        assert!(!response2.success);
        assert!(response2.message.contains("already exists"));
    }

    #[tokio::test]
    async fn test_create_version_nonexistent_config() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let command = RaftCommand::CreateVersion {
            config_id: 999,
            content: b"new content".to_vec(),
            format: Some(ConfigFormat::Json),
            creator_id: 1,
            description: "Version for nonexistent config".to_string(),
        };

        let response = store.apply_command(&command).await.unwrap();
        assert!(!response.success);
        assert!(response.message.contains("not found"));
    }

    #[tokio::test]
    async fn test_update_release_rules_nonexistent_config() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let mut labels = BTreeMap::new();
        labels.insert("env".to_string(), "prod".to_string());
        let releases = vec![Release::new(labels, 1, 10)];

        let command = RaftCommand::UpdateReleaseRules {
            config_id: 999,
            releases,
        };

        let response = store.apply_command(&command).await.unwrap();
        assert!(!response.success);
        assert!(response.message.contains("not found"));
    }

    #[tokio::test]
    async fn test_update_release_rules_nonexistent_version() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        // Create config first
        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "app".to_string(),
            env: "test".to_string(),
        };

        let create_command = RaftCommand::CreateConfig {
            namespace,
            name: "test.json".to_string(),
            content: b"{}".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Test config".to_string(),
        };

        let create_response = store.apply_command(&create_command).await.unwrap();
        assert!(create_response.success);
        let config_id = create_response.data.unwrap()["config_id"].as_u64().unwrap();

        // Try to update release rules with nonexistent version
        let mut labels = BTreeMap::new();
        labels.insert("env".to_string(), "prod".to_string());
        let releases = vec![Release::new(labels, 999, 10)]; // Version 999 doesn't exist

        let command = RaftCommand::UpdateReleaseRules {
            config_id,
            releases,
        };

        let response = store.apply_command(&command).await.unwrap();
        assert!(!response.success);
        assert!(response.message.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_release_version_nonexistent_config() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let command = RaftCommand::ReleaseVersion {
            config_id: 999,
            version_id: 1,
        };

        let response = store.apply_command(&command).await.unwrap();
        assert!(!response.success);
        assert!(response.message.contains("not found"));
    }

    #[tokio::test]
    async fn test_release_version_nonexistent_version() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        // Create config first
        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "app".to_string(),
            env: "test".to_string(),
        };

        let create_command = RaftCommand::CreateConfig {
            namespace,
            name: "test.json".to_string(),
            content: b"{}".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Test config".to_string(),
        };

        let create_response = store.apply_command(&create_command).await.unwrap();
        assert!(create_response.success);
        let config_id = create_response.data.unwrap()["config_id"].as_u64().unwrap();

        // Try to release nonexistent version
        let command = RaftCommand::ReleaseVersion {
            config_id,
            version_id: 999,
        };

        let response = store.apply_command(&command).await.unwrap();
        assert!(!response.success);
        assert!(response.message.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_update_config_command() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        // Create initial config
        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "update".to_string(),
            env: "test".to_string(),
        };

        let create_command = RaftCommand::CreateConfig {
            namespace: namespace.clone(),
            name: "update.json".to_string(),
            content: b"{\"initial\": true}".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Initial config".to_string(),
        };

        let create_response = store.apply_command(&create_command).await.unwrap();
        assert!(create_response.success);
        let config_id = create_response.data.unwrap()["config_id"].as_u64().unwrap();

        // Update the config
        let update_command = RaftCommand::UpdateConfig {
            config_id,
            namespace: namespace.clone(),
            name: "updated.yaml".to_string(),
            content: b"updated: true".to_vec(),
            format: ConfigFormat::Yaml,
            schema: Some("v2".to_string()),
            description: "Updated config".to_string(),
        };

        let update_response = store.apply_command(&update_command).await.unwrap();
        assert!(update_response.success);

        // Verify the config was updated
        let config = store.get_config(&namespace, "updated.yaml").await.unwrap();
        assert_eq!(config.name, "updated.yaml");
        assert_eq!(config.schema, Some("v2".to_string()));
        assert_eq!(config.latest_version_id, 2); // Should have created version 2

        // Verify version was created with new content
        let version = store.get_config_version(config_id, 2).await.unwrap();
        assert_eq!(version.content, b"updated: true");
        assert_eq!(version.format, ConfigFormat::Yaml);
        assert_eq!(version.description, "Updated config");
    }

    #[tokio::test]
    async fn test_update_config_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "app".to_string(),
            env: "test".to_string(),
        };

        let command = RaftCommand::UpdateConfig {
            config_id: 999,
            namespace,
            name: "nonexistent.json".to_string(),
            content: b"{}".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            description: "Nonexistent update".to_string(),
        };

        let response = store.apply_command(&command).await.unwrap();
        assert!(!response.success);
        assert!(response.message.contains("not found"));
    }

    #[tokio::test]
    async fn test_subscribe_changes() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let mut receiver = store.subscribe_changes();

        // Create a config to trigger a change event
        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "subscription".to_string(),
            env: "test".to_string(),
        };

        let command = RaftCommand::CreateConfig {
            namespace: namespace.clone(),
            name: "subscribe.json".to_string(),
            content: b"{}".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Subscription test".to_string(),
        };

        // Apply command in a separate task to avoid blocking
        let store_clone = store.clone();
        tokio::spawn(async move {
            store_clone.apply_command(&command).await.unwrap();
        });

        // Wait for the change notification
        let change_event = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            receiver.recv()
        ).await;

        assert!(change_event.is_ok());
        let event = change_event.unwrap().unwrap();
        assert_eq!(event.namespace, namespace);
        assert_eq!(event.name, "subscribe.json");
        assert_eq!(event.change_type, ConfigChangeType::Created);
    }
}
