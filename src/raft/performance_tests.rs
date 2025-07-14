//! Performance benchmarks for Raft consensus implementation

#[cfg(test)]
mod performance_tests {
    use crate::config::AppConfig;
    use crate::raft::{
        node::{NodeConfig, RaftNode, ResourceLimits},
        types::*,
        validation::RaftInputValidator,
    };
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::time::timeout;
    use uuid::Uuid;

    /// Helper function to create test app config
    async fn create_test_app_config() -> AppConfig {
        // Generate a unique identifier for this test instance
        let test_id = Uuid::new_v4().to_string();

        AppConfig {
            server: crate::config::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                max_connections: 100,
                request_timeout_secs: 30,
            },
            storage: crate::config::StorageConfig {
                data_dir: format!("/tmp/conflux_perf_test_{}", test_id),
                max_open_files: 1000,
                cache_size_mb: 64,
                write_buffer_size_mb: 64,
                max_write_buffer_number: 3,
            },
            ..Default::default()
        }
    }

    /// Helper function to create a test node configuration
    fn create_test_node_config(node_id: NodeId, port: u16) -> NodeConfig {
        NodeConfig {
            node_id,
            address: format!("127.0.0.1:{}", port),
            raft_config: openraft::Config::default(),
            network_config: crate::raft::network::NetworkConfig::default(),
            heartbeat_interval: 50,
            election_timeout_min: 150,
            election_timeout_max: 300,
            resource_limits: ResourceLimits {
                max_requests_per_second: 10000, // High for performance tests
                max_concurrent_requests: 1000,
                max_request_size: 10 * 1024 * 1024,   // 10MB
                max_memory_usage: 1024 * 1024 * 1024, // 1GB
                request_timeout_ms: 10000,
            },
        }
    }

    #[tokio::test]
    async fn benchmark_node_creation_and_startup() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 9000);

        // Benchmark node creation
        let start = Instant::now();
        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        let creation_time = start.elapsed();

        println!("Node creation time: {:?}", creation_time);
        assert!(
            creation_time < Duration::from_millis(100),
            "Node creation should be fast"
        );

        // Benchmark node startup
        let start = Instant::now();
        assert!(node.start().await.is_ok());
        let startup_time = start.elapsed();

        println!("Node startup time: {:?}", startup_time);
        assert!(
            startup_time < Duration::from_secs(5),
            "Node startup should be reasonably fast"
        );

        // Benchmark leadership acquisition
        let start = Instant::now();
        let leadership_result = timeout(
            Duration::from_secs(10),
            node.wait_for_leadership(Duration::from_secs(8)),
        )
        .await;
        let leadership_time = start.elapsed();

        if leadership_result.is_ok() {
            println!("Leadership acquisition time: {:?}", leadership_time);
            assert!(
                leadership_time < Duration::from_secs(5),
                "Leadership should be acquired quickly in single-node cluster"
            );
        }

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn benchmark_validation_performance() {
        let validator = RaftInputValidator::new();

        // Benchmark node ID validation
        let start = Instant::now();
        for i in 1..=10000 {
            let _ = validator
                .comprehensive_validator
                .node_validator
                .validate_node_id(i);
        }
        let node_id_validation_time = start.elapsed();

        println!(
            "10,000 node ID validations time: {:?}",
            node_id_validation_time
        );
        println!(
            "Average per validation: {:?}",
            node_id_validation_time / 10000
        );
        assert!(
            node_id_validation_time < Duration::from_millis(100),
            "Node ID validation should be very fast"
        );

        // Benchmark address validation
        let addresses: Vec<String> = (1..=1000)
            .map(|i| format!("192.168.1.{}:8080", i % 255 + 1))
            .collect();

        let start = Instant::now();
        for address in &addresses {
            let _ = validator
                .comprehensive_validator
                .node_validator
                .validate_node_address(address);
        }
        let address_validation_time = start.elapsed();

        println!(
            "1,000 address validations time: {:?}",
            address_validation_time
        );
        println!(
            "Average per validation: {:?}",
            address_validation_time / 1000
        );
        assert!(
            address_validation_time < Duration::from_millis(500),
            "Address validation should be fast"
        );

        // Benchmark comprehensive node addition validation
        let existing_nodes: Vec<(NodeId, String)> = (1..=100)
            .map(|i| (i, format!("192.168.1.{}:8080", i)))
            .collect();

        let start = Instant::now();
        for i in 101..=200 {
            let address = format!("192.168.1.{}:8080", i);
            let _ = validator.validate_add_node(i, &address, &existing_nodes);
        }
        let comprehensive_validation_time = start.elapsed();

        println!(
            "100 comprehensive validations time: {:?}",
            comprehensive_validation_time
        );
        println!(
            "Average per validation: {:?}",
            comprehensive_validation_time / 100
        );
        assert!(
            comprehensive_validation_time < Duration::from_millis(1000),
            "Comprehensive validation should be reasonably fast"
        );
    }

    #[tokio::test]
    async fn benchmark_metrics_collection() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 9001);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Benchmark basic metrics collection
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = node.get_metrics().await;
        }
        let basic_metrics_time = start.elapsed();

        println!(
            "1,000 basic metrics collections time: {:?}",
            basic_metrics_time
        );
        println!("Average per collection: {:?}", basic_metrics_time / 1000);
        assert!(
            basic_metrics_time < Duration::from_secs(1),
            "Basic metrics should be fast"
        );

        // Benchmark comprehensive metrics collection
        let start = Instant::now();
        for _ in 0..100 {
            let _ = node.get_comprehensive_metrics().await;
        }
        let comprehensive_metrics_time = start.elapsed();

        println!(
            "100 comprehensive metrics collections time: {:?}",
            comprehensive_metrics_time
        );
        println!(
            "Average per collection: {:?}",
            comprehensive_metrics_time / 100
        );
        assert!(
            comprehensive_metrics_time < Duration::from_secs(1),
            "Comprehensive metrics should be reasonably fast"
        );

        // Benchmark node health checks
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = node.get_node_health().await;
        }
        let health_check_time = start.elapsed();

        println!("1,000 health checks time: {:?}", health_check_time);
        println!("Average per check: {:?}", health_check_time / 1000);
        assert!(
            health_check_time < Duration::from_secs(1),
            "Health checks should be fast"
        );

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn benchmark_resource_limiting() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 9002);

        let node = RaftNode::new(node_config, &app_config).await.unwrap();
        let resource_limiter = node.resource_limiter();

        // Benchmark resource limit checks
        let start = Instant::now();
        for _ in 0..10000 {
            let _permit = resource_limiter
                .check_request_allowed(1024, Some("test_client"))
                .await;
        }
        let resource_check_time = start.elapsed();

        println!(
            "10,000 resource limit checks time: {:?}",
            resource_check_time
        );
        println!("Average per check: {:?}", resource_check_time / 10000);
        assert!(
            resource_check_time < Duration::from_secs(1),
            "Resource limit checks should be fast"
        );

        // Benchmark concurrent resource checks
        let start = Instant::now();
        let mut handles = vec![];

        for i in 0..100 {
            let limiter = resource_limiter.clone();
            let handle = tokio::spawn(async move {
                for j in 0..100 {
                    let client_id = format!("client_{}", i);
                    let _permit = limiter.check_request_allowed(1024, Some(&client_id)).await;
                    if j % 10 == 0 {
                        tokio::task::yield_now().await; // Yield occasionally
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let concurrent_check_time = start.elapsed();
        println!(
            "10,000 concurrent resource checks time: {:?}",
            concurrent_check_time
        );
        assert!(
            concurrent_check_time < Duration::from_secs(5),
            "Concurrent resource checks should be reasonably fast"
        );
    }

    #[tokio::test]
    async fn benchmark_timeout_configuration() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 9003);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Benchmark timeout configuration updates
        let start = Instant::now();
        for i in 0..1000 {
            let heartbeat = 50 + (i % 50);
            let min_timeout = 150 + (i % 100);
            let max_timeout = 300 + (i % 200);

            let _ = node
                .update_timeouts(Some(heartbeat), Some(min_timeout), Some(max_timeout))
                .await;
        }
        let timeout_update_time = start.elapsed();

        println!("1,000 timeout updates time: {:?}", timeout_update_time);
        println!("Average per update: {:?}", timeout_update_time / 1000);
        assert!(
            timeout_update_time < Duration::from_secs(2),
            "Timeout updates should be fast"
        );

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn stress_test_node_operations() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 9004);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Stress test with many concurrent operations
        let node_arc = Arc::new(node);
        let start = Instant::now();
        let mut handles = vec![];

        // Spawn multiple tasks doing different operations
        for _i in 0..50 {
            let node_clone = node_arc.clone();
            let handle = tokio::spawn(async move {
                for j in 0..20 {
                    match j % 4 {
                        0 => {
                            let _ = node_clone.get_metrics().await;
                        }
                        1 => {
                            let _ = node_clone.get_node_health().await;
                        }
                        2 => {
                            let _ = node_clone.get_resource_stats();
                        }
                        3 => {
                            let _ = node_clone.get_timeout_config();
                        }
                        _ => unreachable!(),
                    }

                    if j % 5 == 0 {
                        tokio::task::yield_now().await;
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let stress_test_time = start.elapsed();
        println!(
            "Stress test (50 tasks × 20 operations) time: {:?}",
            stress_test_time
        );
        assert!(
            stress_test_time < Duration::from_secs(10),
            "Stress test should complete within reasonable time"
        );

        // Verify node is still functional after stress test
        assert!(node_arc.get_metrics().await.is_ok());
        assert!(node_arc.stop().await.is_ok());
    }

    #[tokio::test]
    async fn memory_usage_benchmark() {
        // Simple memory usage test (would need more sophisticated tools for real measurement)
        let app_config = create_test_app_config().await;
        let _node_config = create_test_node_config(1, 9005);

        let start = Instant::now();
        let mut nodes = vec![];

        // Create multiple nodes to test memory scaling
        for i in 1..=10 {
            let config = create_test_node_config(i, 9010 + i as u16);
            let node = RaftNode::new(config, &app_config).await.unwrap();
            nodes.push(node);
        }

        let creation_time = start.elapsed();
        println!("Created 10 nodes in: {:?}", creation_time);

        // Test that nodes can be created efficiently
        assert!(
            creation_time < Duration::from_secs(5),
            "Multiple node creation should be efficient"
        );

        // Clean up
        for node in nodes {
            let _ = node.stop().await;
        }
    }

    #[tokio::test]
    async fn concurrent_validation_benchmark() {
        let validator = Arc::new(RaftInputValidator::new());

        // Test concurrent validation performance
        let start = Instant::now();
        let mut handles = vec![];

        for i in 0..100 {
            let validator_clone = validator.clone();
            let handle = tokio::spawn(async move {
                for j in 1..=100 {
                    let node_id = i * 100 + j;
                    let address = format!("192.168.{}.{}:8080", i + 1, j);

                    let _ = validator_clone
                        .comprehensive_validator
                        .node_validator
                        .validate_node_id(node_id);
                    let _ = validator_clone
                        .comprehensive_validator
                        .node_validator
                        .validate_node_address(&address);

                    if j % 10 == 0 {
                        tokio::task::yield_now().await;
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let concurrent_validation_time = start.elapsed();
        println!(
            "Concurrent validation (100 tasks × 100 validations) time: {:?}",
            concurrent_validation_time
        );
        assert!(
            concurrent_validation_time < Duration::from_secs(2),
            "Concurrent validation should be fast"
        );
    }

    #[tokio::test]
    async fn large_cluster_simulation_benchmark() {
        let validator = RaftInputValidator::new();

        // Simulate validation for a large cluster
        let start = Instant::now();

        // Create a simulated large cluster
        let mut existing_nodes = vec![];
        for i in 1..=1000 {
            let address = format!("10.0.{}.{}:8080", i / 255, i % 255);
            existing_nodes.push((i, address));
        }

        // Test adding nodes to this large cluster
        for i in 1001..=1100 {
            let address = format!("10.0.{}.{}:8080", i / 255, i % 255);
            let result = validator.validate_add_node(i, &address, &existing_nodes);
            assert!(result.is_err()); // Should fail due to cluster size limit
        }

        let large_cluster_time = start.elapsed();
        println!(
            "Large cluster validation (1000 nodes + 100 additions) time: {:?}",
            large_cluster_time
        );
        assert!(
            large_cluster_time < Duration::from_secs(1),
            "Large cluster validation should be efficient"
        );
    }
}
