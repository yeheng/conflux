#[cfg(test)]
mod tests {
    use crate::raft::store::Store;
    use crate::raft::types::*;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use tempfile::tempdir;

    async fn create_test_store() -> (Arc<Store>, tempfile::TempDir) {
        let temp_dir = tempdir().unwrap();
        let (store, _) = Store::new(temp_dir.path()).await.unwrap();
        (Arc::new(store), temp_dir)
    }

    #[tokio::test]
    async fn test_create_config() {
        let (store, _temp_dir) = create_test_store().await;

        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "myapp".to_string(),
            env: "dev".to_string(),
        };

        let command = RaftCommand::CreateConfig {
            namespace: namespace.clone(),
            name: "database.toml".to_string(),
            content: b"host = \"localhost\"\nport = 5432".to_vec(),
            format: ConfigFormat::Toml,
            schema: None,
            creator_id: 1,
            description: "Initial database config".to_string(),
        };

        let response = store.apply_command(&command).await.unwrap();
        assert!(response.success);

        // Verify config was created
        let config = store.get_config(&namespace, "database.toml").await;
        assert!(config.is_some());

        let config = config.unwrap();
        assert_eq!(config.name, "database.toml");
        assert_eq!(config.namespace, namespace);
        assert_eq!(config.latest_version_id, 1);
    }

    #[tokio::test]
    async fn test_create_version() {
        let (store, _temp_dir) = create_test_store().await;

        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "myapp".to_string(),
            env: "dev".to_string(),
        };

        // First create a config
        let create_command = RaftCommand::CreateConfig {
            namespace: namespace.clone(),
            name: "database.toml".to_string(),
            content: b"host = \"localhost\"\nport = 5432".to_vec(),
            format: ConfigFormat::Toml,
            schema: None,
            creator_id: 1,
            description: "Initial database config".to_string(),
        };

        let response = store.apply_command(&create_command).await.unwrap();
        assert!(response.success);

        let config_id = response.data.unwrap()["config_id"].as_u64().unwrap();

        // Now create a new version
        let version_command = RaftCommand::CreateVersion {
            config_id,
            content: b"host = \"localhost\"\nport = 5433".to_vec(),
            format: Some(ConfigFormat::Toml),
            creator_id: 1,
            description: "Updated port".to_string(),
        };

        let response = store.apply_command(&version_command).await.unwrap();
        assert!(response.success);

        // Verify new version was created
        let config = store.get_config(&namespace, "database.toml").await.unwrap();
        assert_eq!(config.latest_version_id, 2);

        let version = store.get_config_version(config_id, 2).await.unwrap();
        assert_eq!(version.description, "Updated port");
    }

    #[tokio::test]
    async fn test_update_release_rules() {
        let (store, _temp_dir) = create_test_store().await;

        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "myapp".to_string(),
            env: "dev".to_string(),
        };

        // Create a config
        let create_command = RaftCommand::CreateConfig {
            namespace: namespace.clone(),
            name: "database.toml".to_string(),
            content: b"host = \"localhost\"\nport = 5432".to_vec(),
            format: ConfigFormat::Toml,
            schema: None,
            creator_id: 1,
            description: "Initial database config".to_string(),
        };

        let response = store.apply_command(&create_command).await.unwrap();
        let config_id = response.data.unwrap()["config_id"].as_u64().unwrap();

        // Create a second version
        let version_command = RaftCommand::CreateVersion {
            config_id,
            content: b"host = \"localhost\"\nport = 5433".to_vec(),
            format: Some(ConfigFormat::Toml),
            creator_id: 1,
            description: "Updated port".to_string(),
        };
        store.apply_command(&version_command).await.unwrap();

        // Update release rules
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
        assert!(response.success);

        // Verify release rules were updated
        let config = store.get_config(&namespace, "database.toml").await.unwrap();
        assert_eq!(config.releases.len(), 2);
        assert_eq!(config.releases, releases);
    }

    #[tokio::test]
    async fn test_get_published_config() {
        let (store, _temp_dir) = create_test_store().await;

        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "myapp".to_string(),
            env: "dev".to_string(),
        };

        // Create config and versions
        let create_command = RaftCommand::CreateConfig {
            namespace: namespace.clone(),
            name: "database.toml".to_string(),
            content: b"host = \"localhost\"\nport = 5432".to_vec(),
            format: ConfigFormat::Toml,
            schema: None,
            creator_id: 1,
            description: "Initial database config".to_string(),
        };

        let response = store.apply_command(&create_command).await.unwrap();
        let config_id = response.data.unwrap()["config_id"].as_u64().unwrap();

        // Create version 2
        let version_command = RaftCommand::CreateVersion {
            config_id,
            content: b"host = \"localhost\"\nport = 5433".to_vec(),
            format: Some(ConfigFormat::Toml),
            creator_id: 1,
            description: "Production version".to_string(),
        };
        store.apply_command(&version_command).await.unwrap();

        // Set up release rules
        let mut prod_labels = BTreeMap::new();
        prod_labels.insert("env".to_string(), "production".to_string());

        let releases = vec![
            Release::new(prod_labels, 2, 10), // Production gets version 2
            Release::default(1),              // Default gets version 1
        ];

        let update_command = RaftCommand::UpdateReleaseRules {
            config_id,
            releases,
        };
        store.apply_command(&update_command).await.unwrap();

        // Test production client
        let mut prod_client_labels = BTreeMap::new();
        prod_client_labels.insert("env".to_string(), "production".to_string());

        let (_config, version) = store
            .get_published_config(&namespace, "database.toml", &prod_client_labels)
            .await
            .unwrap();

        assert_eq!(version.id, 2);
        assert_eq!(version.description, "Production version");

        // Test default client
        let dev_client_labels = BTreeMap::new();

        let (_config, version) = store
            .get_published_config(&namespace, "database.toml", &dev_client_labels)
            .await
            .unwrap();

        assert_eq!(version.id, 1);
        assert_eq!(version.description, "Initial database config");
    }

    #[tokio::test]
    async fn test_config_version_integrity() {
        let content = b"test content";
        let version = ConfigVersion::new(
            1,
            1,
            content.to_vec(),
            ConfigFormat::Json,
            1,
            "Test version".to_string(),
        );

        assert!(version.verify_integrity());

        // Test with modified content
        let mut modified_version = version.clone();
        modified_version.content = b"modified content".to_vec();
        assert!(!modified_version.verify_integrity());
    }

    #[tokio::test]
    async fn test_release_matching() {
        let mut labels = BTreeMap::new();
        labels.insert("env".to_string(), "production".to_string());
        labels.insert("region".to_string(), "us-east-1".to_string());

        let release = Release::new(labels, 1, 10);

        // Test exact match
        let mut client_labels = BTreeMap::new();
        client_labels.insert("env".to_string(), "production".to_string());
        client_labels.insert("region".to_string(), "us-east-1".to_string());
        client_labels.insert("extra".to_string(), "value".to_string());

        assert!(release.matches(&client_labels));

        // Test partial match (should fail)
        let mut partial_labels = BTreeMap::new();
        partial_labels.insert("env".to_string(), "production".to_string());

        assert!(!release.matches(&partial_labels));

        // Test default release
        let default_release = Release::default(1);
        assert!(default_release.is_default());
        assert!(default_release.matches(&BTreeMap::new()));
    }
}
