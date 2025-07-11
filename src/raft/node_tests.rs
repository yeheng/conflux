#[cfg(test)]
mod tests {
    use crate::raft::node::*;
    use crate::raft::network::NetworkConfig;
    use crate::config::AppConfig;
    use openraft::Config as RaftConfig;
    use std::collections::{BTreeSet, HashMap};
    use std::time::Duration;
    use tempfile::TempDir;

    /// Create a test node config
    fn create_test_node_config() -> NodeConfig {
        let mut addresses = HashMap::new();
        addresses.insert(1, "127.0.0.1:8001".to_string());
        addresses.insert(2, "127.0.0.1:8002".to_string());

        NodeConfig {
            node_id: 1,
            address: "127.0.0.1:8001".to_string(),
            raft_config: RaftConfig::default(),
            network_config: NetworkConfig::new(addresses),
            heartbeat_interval: 150,
            election_timeout_min: 300,
            election_timeout_max: 600,
            resource_limits: ResourceLimits::default(),
        }
    }

    /// Create a test app config with temporary directory
    fn create_test_app_config() -> (AppConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AppConfig::default();
        config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
        (config, temp_dir)
    }

    #[tokio::test]
    async fn test_node_config_creation() {
        let config = NodeConfig::default();
        assert_eq!(config.node_id, 1);
        assert_eq!(config.address, "127.0.0.1:8080");
        // Note: RaftConfig doesn't have an 'id' field in openraft 0.9
    }

    #[tokio::test]
    async fn test_node_config_custom() {
        let config = create_test_node_config();
        assert_eq!(config.node_id, 1);
        assert_eq!(config.address, "127.0.0.1:8001");

        let addresses = config.network_config.node_addresses.read().await;
        assert_eq!(addresses.len(), 2);
        assert!(addresses.contains_key(&1));
        assert!(addresses.contains_key(&2));
    }

    #[tokio::test]
    async fn test_create_node_config_helper() {
        let config = create_node_config(42, "192.168.1.100:9000".to_string());
        assert_eq!(config.node_id, 42);
        assert_eq!(config.address, "192.168.1.100:9000");
    }

    #[tokio::test]
    async fn test_raft_node_creation() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let result = RaftNode::new(node_config, &app_config).await;
        assert!(result.is_ok());

        let node = result.unwrap();
        assert_eq!(node.node_id(), 1);
        assert_eq!(node.address(), "127.0.0.1:8001");
    }

    #[tokio::test]
    async fn test_raft_node_basic_properties() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Test basic getters
        assert_eq!(node.node_id(), 1);
        assert_eq!(node.address(), "127.0.0.1:8001");

        // Test store access
        let store_ref = node.store();
        assert!(store_ref.configurations.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_raft_node_members_management() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Initially should have self as member
        let members = node.get_members().await;
        assert_eq!(members.len(), 1);
        assert!(members.contains(&1));

        // Add a node
        let result = node.add_node(2, "127.0.0.1:8002".to_string()).await;
        assert!(result.is_ok());

        // Check members updated
        let members = node.get_members().await;
        assert_eq!(members.len(), 2);
        assert!(members.contains(&1));
        assert!(members.contains(&2));

        // Remove the node
        let result = node.remove_node(2).await;
        assert!(result.is_ok());

        // Check members updated
        let members = node.get_members().await;
        assert_eq!(members.len(), 1);
        assert!(members.contains(&1));
    }

    #[tokio::test]
    async fn test_raft_node_leadership_status() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Initially should not be leader (no Raft instance started)
        let is_leader = node.is_leader().await;
        assert!(!is_leader);

        // Should have no leader
        let leader = node.get_leader().await;
        assert_eq!(leader, None);
    }

    #[tokio::test]
    async fn test_raft_node_raft_instance_access() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Initially should have no Raft instance
        let raft_ref = node.get_raft();
        assert!(raft_ref.is_none());
    }

    #[tokio::test]
    async fn test_raft_node_start_stop() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Test start - should succeed
        let start_result = node.start().await;
        assert!(start_result.is_ok());

        // Test stop
        let stop_result = node.stop().await;
        assert!(stop_result.is_ok());
    }

    #[tokio::test]
    async fn test_raft_node_metrics() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Test metrics access - should fail gracefully when no Raft instance
        let metrics_result = node.get_metrics().await;
        // We expect this to fail since no Raft instance is running
        assert!(metrics_result.is_err());
    }

    #[tokio::test]
    async fn test_raft_node_wait_for_leadership() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Test waiting for leadership with short timeout - should timeout since no Raft instance
        let timeout = Duration::from_millis(100);
        let leadership_result = node.wait_for_leadership(timeout).await;

        // Should timeout since no Raft instance is running
        assert!(leadership_result.is_err());
    }

    #[tokio::test]
    async fn test_raft_node_change_membership() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Test membership change
        let mut new_members = BTreeSet::new();
        new_members.insert(1);
        new_members.insert(2);
        new_members.insert(3);

        let membership_result = node.change_membership(new_members.clone()).await;
        // Should fail since no Raft instance is running
        assert!(membership_result.is_err());

        // Check membership was not updated (still just self)
        let members = node.get_members().await;
        assert_eq!(members.len(), 1);
        assert!(members.contains(&1));
    }

    #[tokio::test]
    async fn test_raft_node_multiple_add_remove_operations() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Add multiple nodes
        for i in 2..=5 {
            let result = node.add_node(i, format!("127.0.0.1:800{}", i)).await;
            assert!(result.is_ok());
        }

        let members = node.get_members().await;
        assert_eq!(members.len(), 5); // Nodes 1, 2, 3, 4, 5

        // Remove some nodes
        let result = node.remove_node(3).await;
        assert!(result.is_ok());
        let result = node.remove_node(5).await;
        assert!(result.is_ok());

        let members = node.get_members().await;
        assert_eq!(members.len(), 3); // Nodes 1, 2, 4
        assert!(members.contains(&1));
        assert!(members.contains(&2));
        assert!(members.contains(&4));
        assert!(!members.contains(&3));
        assert!(!members.contains(&5));
    }

    #[tokio::test]
    async fn test_raft_node_duplicate_add_operations() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Add a node
        let result1 = node.add_node(2, "127.0.0.1:8002".to_string()).await;
        assert!(result1.is_ok());

        // Add the same node again
        let result2 = node.add_node(2, "127.0.0.1:8002".to_string()).await;
        assert!(result2.is_ok()); // Should not fail, just update

        let members = node.get_members().await;
        assert_eq!(members.len(), 2); // Node 1 and 2
        assert!(members.contains(&1));
        assert!(members.contains(&2));
    }

    #[tokio::test]
    async fn test_raft_node_remove_nonexistent_node() {
        let node_config = create_test_node_config();
        let (app_config, _temp_dir) = create_test_app_config();

        let node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Try to remove a node that doesn't exist
        let result = node.remove_node(999).await;
        assert!(result.is_ok()); // Should not fail, just be a no-op

        let members = node.get_members().await;
        assert_eq!(members.len(), 1); // Still has self
        assert!(members.contains(&1));
    }
}
