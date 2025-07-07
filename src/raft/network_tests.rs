#[cfg(test)]
mod tests {
    use crate::raft::network::{NetworkConfig, ConfluxNetwork, ConfluxNetworkFactory};
    use openraft::{
        network::RaftNetworkFactory,
        BasicNode,
    };
    use std::collections::HashMap;

    /// Create a test network config
    fn create_test_network_config() -> NetworkConfig {
        let mut addresses = HashMap::new();
        addresses.insert(1, "127.0.0.1:8001".to_string());
        addresses.insert(2, "127.0.0.1:8002".to_string());
        addresses.insert(3, "127.0.0.1:8003".to_string());

        NetworkConfig::new(addresses)
    }

    #[tokio::test]
    async fn test_network_config_creation() {
        let config = NetworkConfig::default();
        assert_eq!(config.timeout_secs, 10);
        assert!(config.node_addresses.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_network_config_with_addresses() {
        let config = create_test_network_config();
        assert_eq!(config.timeout_secs, 10);

        let addresses = config.node_addresses.read().await;
        assert_eq!(addresses.len(), 3);
        assert_eq!(addresses.get(&1), Some(&"127.0.0.1:8001".to_string()));
        assert_eq!(addresses.get(&2), Some(&"127.0.0.1:8002".to_string()));
        assert_eq!(addresses.get(&3), Some(&"127.0.0.1:8003".to_string()));
    }

    #[tokio::test]
    async fn test_network_config_add_node() {
        let config = NetworkConfig::default();
        config.add_node(1, "127.0.0.1:8001".to_string()).await;

        let addresses = config.node_addresses.read().await;
        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses.get(&1), Some(&"127.0.0.1:8001".to_string()));
    }

    #[tokio::test]
    async fn test_network_config_get_node_address() {
        let config = create_test_network_config();

        let address = config.get_node_address(1).await;
        assert_eq!(address, Some("127.0.0.1:8001".to_string()));

        let missing_address = config.get_node_address(999).await;
        assert_eq!(missing_address, None);
    }

    #[tokio::test]
    async fn test_conflux_network_factory_creation() {
        let config = create_test_network_config();
        let mut factory = ConfluxNetworkFactory::new(config);

        // Test that we can create a network instance
        let network = factory.new_client(1, &BasicNode::default()).await;
        assert_eq!(network.target_node_id, 1);
    }

    #[tokio::test]
    async fn test_conflux_network_creation() {
        let config = create_test_network_config();
        let network = ConfluxNetwork::new(config, 1);

        assert_eq!(network.target_node_id, 1);
    }

    #[tokio::test]
    async fn test_conflux_network_is_reachable() {
        let config = create_test_network_config();
        let network = ConfluxNetwork::new(config, 1);

        // This will likely fail since we don't have actual servers running
        // but we're testing the method exists and returns a boolean
        let reachable = network.is_reachable().await;
        assert!(!reachable); // Expected to be false in test environment
    }

    #[tokio::test]
    async fn test_conflux_network_connection_stats() {
        let config = create_test_network_config();
        let network = ConfluxNetwork::new(config, 1);

        let stats = network.get_connection_stats().await;
        assert_eq!(stats.target_node_id, 1);
        assert_eq!(stats.timeout_secs, 10);
        assert!(!stats.is_reachable); // Expected to be false in test environment
    }

    #[tokio::test]
    async fn test_conflux_network_missing_address() {
        let config = NetworkConfig::default(); // Empty config
        let network = ConfluxNetwork::new(config, 999); // Non-existent node

        // Test that connection stats work even with missing address
        let stats = network.get_connection_stats().await;
        assert_eq!(stats.target_node_id, 999);
        assert!(!stats.is_reachable);
    }

    #[tokio::test]
    async fn test_multiple_network_instances() {
        let config = create_test_network_config();
        let mut factory = ConfluxNetworkFactory::new(config);

        // Create multiple network instances for different nodes
        let network1 = factory.new_client(1, &BasicNode::default()).await;
        let network2 = factory.new_client(2, &BasicNode::default()).await;
        let network3 = factory.new_client(3, &BasicNode::default()).await;

        // Verify they target different nodes
        assert_eq!(network1.target_node_id, 1);
        assert_eq!(network2.target_node_id, 2);
        assert_eq!(network3.target_node_id, 3);
    }
}
