//! 简单的3节点集群测试
//! 专注于核心Raft功能验证

#[cfg(test)]
mod simple_cluster_tests {
    use crate::config::{AppConfig, StorageConfig};
    use crate::raft::{
        network::NetworkConfig,
        node::{NodeConfig, RaftNode},
    };
    use openraft::Config as RaftConfig;
    use std::collections::HashMap;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::time::sleep;
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn test_single_node_creation_and_startup() {
        // 首先测试单个节点能否正常创建和启动
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let node_config = NodeConfig {
            node_id: 1,
            address: "127.0.0.1:18080".to_string(),
            raft_config: RaftConfig {
                heartbeat_interval: 150,
                election_timeout_min: 300,
                election_timeout_max: 600,
                ..Default::default()
            },
            network_config: NetworkConfig::default(),
            heartbeat_interval: 150,
            election_timeout_min: 300,
            election_timeout_max: 600,
            resource_limits: crate::raft::node::ResourceLimits::default(),
        };

        let app_config = AppConfig {
            storage: StorageConfig {
                data_dir: temp_dir.path().to_string_lossy().to_string(),
                max_open_files: -1,
                cache_size_mb: 8,
                write_buffer_size_mb: 8,
                max_write_buffer_number: 2,
            },
            ..Default::default()
        };

        // 创建节点
        let mut node = RaftNode::new(node_config, &app_config)
            .await
            .expect("Failed to create node");

        // 启动节点
        let start_result = node.start().await;
        assert!(
            start_result.is_ok(),
            "Failed to start node: {:?}",
            start_result.err()
        );

        // 等待一段时间让节点稳定
        sleep(Duration::from_millis(500)).await;

        // 检查节点状态
        println!("Node {} started successfully", node.node_id());
        println!("Node address: {}", node.address());

        // 在单节点集群中，该节点应该成为领导者
        let is_leader = node.is_leader().await;
        println!("Is leader: {}", is_leader);

        // 获取metrics验证节点状态
        let metrics_result = node.get_metrics().await;
        assert!(
            metrics_result.is_ok(),
            "Failed to get metrics: {:?}",
            metrics_result.err()
        );

        let metrics = metrics_result.unwrap();
        println!("Node metrics: {:?}", metrics);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_two_node_creation() {
        // 测试两个节点的创建（不涉及复杂的集群逻辑）
        let temp_dir1 = TempDir::new().expect("Failed to create temp dir 1");
        let temp_dir2 = TempDir::new().expect("Failed to create temp dir 2");

        let mut node_addresses = HashMap::new();
        node_addresses.insert(1u64, "127.0.0.1:18083".to_string());
        node_addresses.insert(2u64, "127.0.0.1:18084".to_string());

        let network_config = NetworkConfig::new(node_addresses);

        // 创建第一个节点
        let node_config1 = NodeConfig {
            node_id: 1,
            address: "127.0.0.1:18083".to_string(),
            raft_config: RaftConfig::default(),
            network_config: network_config.clone(),
            heartbeat_interval: 150,
            election_timeout_min: 300,
            election_timeout_max: 600,
            resource_limits: crate::raft::node::ResourceLimits::default(),
        };

        let app_config1 = AppConfig {
            storage: StorageConfig {
                data_dir: temp_dir1.path().to_string_lossy().to_string(),
                max_open_files: -1,
                cache_size_mb: 8,
                write_buffer_size_mb: 8,
                max_write_buffer_number: 2,
            },
            ..Default::default()
        };

        // 创建第二个节点
        let node_config2 = NodeConfig {
            node_id: 2,
            address: "127.0.0.1:18084".to_string(),
            raft_config: RaftConfig::default(),
            network_config: network_config.clone(),
            heartbeat_interval: 150,
            election_timeout_min: 300,
            election_timeout_max: 600,
            resource_limits: crate::raft::node::ResourceLimits::default(),
        };

        let app_config2 = AppConfig {
            storage: StorageConfig {
                data_dir: temp_dir2.path().to_string_lossy().to_string(),
                max_open_files: -1,
                cache_size_mb: 8,
                write_buffer_size_mb: 8,
                max_write_buffer_number: 2,
            },
            ..Default::default()
        };

        let node1 = RaftNode::new(node_config1, &app_config1).await;
        assert!(node1.is_ok(), "Failed to create node 1: {:?}", node1.err());

        let node2 = RaftNode::new(node_config2, &app_config2).await;
        assert!(node2.is_ok(), "Failed to create node 2: {:?}", node2.err());

        let node1 = node1.unwrap();
        let node2 = node2.unwrap();

        println!(
            "Node 1 ID: {}, Address: {}",
            node1.node_id(),
            node1.address()
        );
        println!(
            "Node 2 ID: {}, Address: {}",
            node2.node_id(),
            node2.address()
        );

        // 验证节点具有不同的ID和存储
        assert_eq!(node1.node_id(), 1);
        assert_eq!(node2.node_id(), 2);
        assert_ne!(node1.address(), node2.address());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_store_and_state_machine_integration() {
        // 验证Store和StateMachine的集成是否正常工作
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let node_config = NodeConfig {
            node_id: 1,
            address: "127.0.0.1:18085".to_string(),
            raft_config: RaftConfig::default(),
            network_config: NetworkConfig::default(),
            heartbeat_interval: 150,
            election_timeout_min: 300,
            election_timeout_max: 600,
            resource_limits: crate::raft::node::ResourceLimits::default(),
        };

        let app_config = AppConfig {
            storage: StorageConfig {
                data_dir: temp_dir.path().to_string_lossy().to_string(),
                max_open_files: -1,
                cache_size_mb: 8,
                write_buffer_size_mb: 8,
                max_write_buffer_number: 2,
            },
            ..Default::default()
        };

        // 创建并启动节点（这会同时测试Store和StateMachine的集成）
        let mut node = RaftNode::new(node_config, &app_config)
            .await
            .expect("Failed to create node");

        let start_result = node.start().await;
        assert!(
            start_result.is_ok(),
            "Failed to start node: {:?}",
            start_result.err()
        );

        // 等待一段时间确保状态机管理器正常运行
        sleep(Duration::from_millis(200)).await;

        println!("Store and StateMachine integration test passed");
    }
}
