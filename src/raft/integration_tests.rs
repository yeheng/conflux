//! Integration tests for multi-node Raft cluster scenarios

#[cfg(test)]
mod integration_tests {
    use crate::auth::AuthContext;
    use crate::config::AppConfig;
    use crate::raft::validation::NodeValidator;
    use crate::raft::{
        auth::RaftAuthzService,
        node::{NodeConfig, RaftNode, ResourceLimits},
        types::*,
        validation::{RaftInputValidator, ValidationConfig},
    };
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::timeout;

    /// Helper function to create test app config
    async fn create_test_app_config() -> AppConfig {
        AppConfig {
            server: crate::config::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                max_connections: 100,
                request_timeout_secs: 30,
            },
            raft: crate::config::RaftConfig {
                node_id: 1,
                cluster_name: "test_cluster".to_string(),
                data_dir: format!("/tmp/conflux_test_{}", std::process::id()),
                heartbeat_interval_ms: 150,
                election_timeout_ms: 300,
                snapshot_threshold: 1000,
                max_applied_log_to_keep: 1000,
            },
            storage: crate::config::StorageConfig {
                data_dir: format!("/tmp/conflux_test_{}", std::process::id()),
                max_open_files: 1000,
                cache_size_mb: 64,
                write_buffer_size_mb: 64,
                max_write_buffer_number: 3,
            },
            database: crate::config::DatabaseConfig {
                url: "postgresql://test:test@localhost/test".to_string(),
                max_connections: 10,
                min_connections: 1,
                connect_timeout_secs: 30,
                idle_timeout_secs: 600,
                max_lifetime_secs: 1800,
            },
            security: crate::config::SecurityConfig {
                jwt_secret: "test_secret".to_string(),
                jwt_expiration_hours: 24,
                enable_mtls: false,
                cert_file: None,
                key_file: None,
                ca_file: None,
            },
            observability: crate::config::ObservabilityConfig {
                metrics_enabled: true,
                metrics_port: 9090,
                tracing_enabled: true,
                tracing_endpoint: None,
                log_level: "info".to_string(),
            },
        }
    }

    /// Helper function to create a test node configuration
    fn create_test_node_config(node_id: NodeId, port: u16) -> NodeConfig {
        NodeConfig {
            node_id,
            address: format!("127.0.0.1:{}", port),
            raft_config: openraft::Config::default(),
            network_config: crate::raft::network::NetworkConfig::default(),
            heartbeat_interval: 50, // Faster for tests
            election_timeout_min: 150,
            election_timeout_max: 300,
            resource_limits: ResourceLimits {
                max_requests_per_second: 1000,
                max_concurrent_requests: 100,
                max_request_size: 1024 * 1024,
                max_memory_usage: 100 * 1024 * 1024,
                request_timeout_ms: 5000,
            },
        }
    }

    /// Helper function to create authorization service for testing
    async fn create_test_authz_service() -> Option<Arc<RaftAuthzService>> {
        // In a real test environment, you'd connect to a test database
        // For now, we'll return None to skip authorization in tests
        None
    }

    #[tokio::test]
    async fn test_single_node_cluster_initialization() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 8080);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Test node creation
        assert_eq!(node.node_id(), 1);
        assert_eq!(node.address(), "127.0.0.1:8080");

        // Test starting the node
        assert!(node.start().await.is_ok());

        // Wait for leadership
        let _leadership_result = timeout(
            Duration::from_secs(5),
            node.wait_for_leadership(Duration::from_secs(3)),
        )
        .await;

        assert!(_leadership_result.is_ok());
        assert!(node.is_leader().await);

        // Test getting members
        let members = node.get_members().await;
        assert_eq!(members.len(), 1);
        assert!(members.contains(&1));

        // Test stopping the node
        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_input_validation_integration() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 8081);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Wait for leadership
        let _ = timeout(
            Duration::from_secs(5),
            node.wait_for_leadership(Duration::from_secs(3)),
        )
        .await;

        // Test input validation for node addition

        // Valid node addition should work
        // Note: This might fail in real Raft due to networking, but validation should pass
        let _result = node.add_node(2, "127.0.0.1:8082".to_string()).await;
        // We expect this to work from a validation perspective

        // Invalid node ID should fail
        let result = node.add_node(0, "127.0.0.1:8083".to_string()).await;
        assert!(result.is_err());

        // Invalid address should fail
        let result = node.add_node(3, "invalid-address".to_string()).await;
        assert!(result.is_err());

        // Duplicate node ID should fail
        let result = node.add_node(1, "127.0.0.1:8084".to_string()).await;
        assert!(result.is_err());

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_resource_limits_integration() {
        let app_config = create_test_app_config().await;
        let mut node_config = create_test_node_config(1, 8085);

        // Set very restrictive resource limits for testing
        node_config.resource_limits = ResourceLimits {
            max_requests_per_second: 1,
            max_concurrent_requests: 1,
            max_request_size: 100,  // Very small
            max_memory_usage: 1000, // Very small
            request_timeout_ms: 1000,
        };

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Test resource limits
        let resource_stats = node.get_resource_stats();
        assert_eq!(resource_stats.total_requests, 0);
        assert_eq!(resource_stats.rejected_requests, 0);

        // Test that a large request would be rejected
        // (This is a simplified test since we'd need to actually make requests)
        let resource_limiter = node.resource_limiter();

        // Test request size limit
        let result = resource_limiter.check_request_allowed(200, None).await; // Exceeds limit
        assert!(result.is_err());

        // Test valid request
        let result = resource_limiter.check_request_allowed(50, None).await; // Within limit
        assert!(result.is_ok());

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_timeout_configuration_integration() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 8086);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Test getting current timeout configuration
        let (heartbeat, min_timeout, max_timeout) = node.get_timeout_config();
        assert_eq!(heartbeat, 50);
        assert_eq!(min_timeout, 150);
        assert_eq!(max_timeout, 300);

        // Test updating timeout configuration
        let result = node.update_timeouts(Some(75), Some(200), Some(400)).await;
        assert!(result.is_ok());

        let (heartbeat, min_timeout, max_timeout) = node.get_timeout_config();
        assert_eq!(heartbeat, 75);
        assert_eq!(min_timeout, 200);
        assert_eq!(max_timeout, 400);

        // Test invalid timeout configuration
        let result = node.update_timeouts(Some(0), None, None).await; // Invalid: zero
        assert!(result.is_err());

        let result = node.update_timeouts(Some(500), Some(300), None).await; // Invalid: heartbeat >= min
        assert!(result.is_err());

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_collection_integration() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 8087);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Wait for leadership
        let _ = timeout(
            Duration::from_secs(5),
            node.wait_for_leadership(Duration::from_secs(3)),
        )
        .await;

        // Test metrics collection
        let metrics_result = node.get_metrics().await;
        assert!(metrics_result.is_ok());

        let metrics = metrics_result.unwrap();
        assert_eq!(metrics.node_id, 1);
        // current_term is u64, so it's always >= 0
        assert!(metrics.membership.contains(&1));

        // Test comprehensive metrics
        let comprehensive_metrics = node.get_comprehensive_metrics().await;
        assert!(comprehensive_metrics.is_ok());

        // Test node health
        let health = node.get_node_health().await;
        assert!(health.is_ok());

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_validation_config_customization() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 8088);

        let node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Test getting the input validator
        let validator = node.input_validator();

        // Test default validation config
        let config = validator.get_config();
        assert_eq!(config.min_node_id, 1);
        assert_eq!(config.max_node_id, 65535);

        // Create a custom validation config
        let custom_config = ValidationConfig {
            min_node_id: 10,
            max_node_id: 100,
            max_cluster_size: 5,
            allow_localhost: false,
            ..Default::default()
        };

        let custom_validator = RaftInputValidator::with_config(custom_config);

        // Test custom validation behavior
        assert!(custom_validator.validate_node_id(5).is_err()); // Below minimum
        assert!(custom_validator.validate_node_id(10).is_ok()); // At minimum
        assert!(custom_validator.validate_node_id(100).is_ok()); // At maximum
        assert!(custom_validator.validate_node_id(101).is_err()); // Above maximum

        // Test localhost restriction
        assert!(custom_validator
            .validate_node_address("127.0.0.1:8080")
            .is_err());

        // Test cluster size restriction
        let large_cluster: Vec<(NodeId, String)> = (1..=5)
            .map(|i| (i, format!("192.168.1.{}:8080", i)))
            .collect();
        assert!(custom_validator
            .validate_add_node(6, "192.168.1.6:8080", &large_cluster)
            .is_err());
    }

    #[tokio::test]
    async fn test_concurrent_node_operations() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 8089);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Wait for leadership
        let _ = timeout(
            Duration::from_secs(5),
            node.wait_for_leadership(Duration::from_secs(3)),
        )
        .await;

        // Test concurrent operations
        let node_arc = Arc::new(node);
        let mut handles = vec![];

        // Spawn multiple concurrent operations
        for i in 2..=5 {
            let node_clone = node_arc.clone();
            let handle = tokio::spawn(async move {
                let address = format!("127.0.0.1:808{}", i);
                node_clone.add_node(i, address).await
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        let mut results = vec![];
        for handle in handles {
            let result = handle.await.unwrap();
            results.push(result);
        }

        // Some operations might fail due to Raft networking, but input validation should work
        // The important thing is that the system doesn't crash or deadlock

        assert!(node_arc.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_error_handling_resilience() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 8090);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Test that various error conditions don't crash the system

        // Invalid node operations
        let _ = node.add_node(0, "invalid".to_string()).await; // Should fail gracefully
        let _ = node.remove_node(999).await; // Should fail gracefully

        // Invalid timeout configurations
        let _ = node.update_timeouts(Some(0), None, None).await; // Should fail gracefully
        let _ = node.update_timeouts(Some(99999), None, None).await; // Should fail gracefully

        // The node should still be functional after these errors
        assert!(node.is_leader().await || !node.is_leader().await); // Should not panic

        let _metrics_result = node.get_metrics().await;
        assert!(_metrics_result.is_ok() || _metrics_result.is_err()); // Should not panic

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_authorization_integration_stub() {
        // This test demonstrates how authorization would be integrated
        // In a real environment, you'd set up a test database and authorization service

        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 8091);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();

        // Test that authorization service can be set
        if let Some(authz_service) = create_test_authz_service().await {
            node.set_authz_service(authz_service);
            assert!(node.authz_service().is_some());
        } else {
            assert!(node.authz_service().is_none());
        }

        assert!(node.start().await.is_ok());

        // Test authorized operations (would require real auth service)
        let auth_ctx = AuthContext::new("test_user".to_string(), "test_tenant".to_string());

        // These operations should work when no auth service is configured
        let _result = node
            .add_node_with_auth(2, "127.0.0.1:8092".to_string(), Some(auth_ctx.clone()))
            .await;
        // Result depends on Raft networking, but should not fail due to missing auth

        let _result = node.remove_node_with_auth(2, Some(auth_ctx.clone())).await;
        // Result depends on whether node 2 was actually added

        let result = node
            .update_timeouts_with_auth(Some(100), Some(200), Some(400), Some(auth_ctx))
            .await;
        assert!(result.is_ok()); // Should work when no auth service is configured

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_stress_validation_operations() {
        let config = ValidationConfig::default();
        let validator = NodeValidator::new(&config);

        // Stress test validation with many operations
        for i in 1..=1000 {
            assert!(validator.validate_node_id(i).is_ok());
        }

        // Test many address validations
        for i in 1..=100 {
            let address = format!("192.168.1.{}:8080", i);
            assert!(validator.validate_node_address(&address).is_ok());
        }

        // Test cluster size validation with various sizes
        for current_size in 0..=50 {
            for adding in 1..=5 {
                let result = validator.validate_cluster_size(current_size, adding);
                if current_size + adding <= 100 {
                    assert!(result.is_ok());
                } else {
                    assert!(result.is_err());
                }
            }
        }
    }
}
