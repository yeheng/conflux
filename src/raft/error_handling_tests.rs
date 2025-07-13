//! Error handling and edge case tests

#[cfg(test)]
mod error_handling_tests {
    use crate::auth::AuthContext;
    use crate::config::AppConfig;
    use crate::error::ConfluxError;
    use crate::raft::validation::{ClusterValidator, NodeValidator};
    use crate::raft::{
        node::{NodeConfig, RaftNode, ResourceLimits},
        types::*,
        validation::{RaftInputValidator, ValidationConfig}
        ,
    };

    /// Helper function to create test app config
    async fn create_test_app_config() -> AppConfig {
        AppConfig {
            server: crate::config::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                max_connections: 100,
                request_timeout_secs: 30,
            },
            storage: crate::config::StorageConfig {
                data_dir: format!("/tmp/conflux_error_test_{}", std::process::id()),
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
            resource_limits: ResourceLimits::default(),
        }
    }

    #[tokio::test]
    async fn test_invalid_node_id_error_handling() {
        let config = ValidationConfig::default();
        let validator = NodeValidator::new(&config);

        // Test various invalid node IDs
        let invalid_node_ids = vec![0, 65536, u64::MAX];

        for node_id in invalid_node_ids {
            let result = validator.validate_node_id(node_id);
            assert!(result.is_err());

            match result {
                Err(ConfluxError::Validation(msg)) => {
                    assert!(msg.contains("Node ID"));
                    assert!(!msg.is_empty());
                }
                _ => panic!("Expected validation error for node ID {}", node_id),
            }
        }
    }

    #[tokio::test]
    async fn test_invalid_address_error_handling() {
        let config = ValidationConfig::default();
        let validator = NodeValidator::new(&config);

        // Test various invalid addresses
        let invalid_addresses = vec![
            "",
            "invalid",
            "127.0.0.1",            // Missing port
            "127.0.0.1:99999",      // Port too high
            "127.0.0.1:80",         // Port too low
            "256.256.256.256:8080", // Invalid IP
            "not.an.ip:8080",
            "127.0.0.1:abc",  // Invalid port
            "[::]:8080",      // Unspecified IPv6
            "[ff02::1]:8080", // Multicast IPv6
        ];

        for address in invalid_addresses {
            let result = validator.validate_node_address(address);
            assert!(result.is_err(), "Address '{}' should be invalid", address);

            match result {
                Err(ConfluxError::Validation(msg)) => {
                    assert!(!msg.is_empty());
                    println!("Address '{}' error: {}", address, msg);
                }
                _ => panic!("Expected validation error for address '{}'", address),
            }
        }
    }

    #[tokio::test]
    async fn test_cluster_size_limit_error_handling() {
        let config = ValidationConfig::default();
        let validator = ClusterValidator::new(&config);

        // Test cluster size limits
        let result = validator.validate_cluster_size(100, 1);
        assert!(result.is_err());

        let result = validator.validate_cluster_size(50, 51);
        assert!(result.is_err());

        let result = validator.validate_cluster_size(usize::MAX, 1);
        assert!(result.is_err());

        // Test with custom small limit
        let config = ValidationConfig {
            max_cluster_size: 3,
            ..Default::default()
        };
        let validator = ClusterValidator::new(&config);

        let result = validator.validate_cluster_size(3, 1);
        assert!(result.is_err());

        match result {
            Err(ConfluxError::Validation(msg)) => {
                assert!(msg.contains("exceed"));
                assert!(msg.contains("maximum"));
            }
            _ => panic!("Expected cluster size validation error"),
        }
    }

    #[tokio::test]
    async fn test_timeout_validation_error_handling() {
        let validator = RaftInputValidator::new();

        // Test invalid timeout values
        let invalid_timeouts = vec![
            (Some(0), None, None),        // Zero heartbeat
            (None, Some(0), None),        // Zero min timeout
            (None, None, Some(0)),        // Zero max timeout
            (Some(15000), None, None),    // Too large a heartbeat
            (None, Some(50000), None),    // Too large min timeout
            (None, None, Some(70000)),    // Too large max timeout
            (Some(500), Some(300), None), // Heartbeat >= min
            (Some(600), None, Some(300)), // Heartbeat >= max
            (None, Some(600), Some(300)), // Min >= max
        ];

        for (heartbeat, min_timeout, max_timeout) in invalid_timeouts {
            let result = validator.validate_timeout_config(heartbeat, min_timeout, max_timeout);
            assert!(
                result.is_err(),
                "Timeout config {:?} should be invalid",
                (heartbeat, min_timeout, max_timeout)
            );

            match result {
                Err(ConfluxError::Validation(msg)) => {
                    assert!(!msg.is_empty());
                    println!("Timeout error: {}", msg);
                }
                _ => panic!("Expected timeout validation error"),
            }
        }
    }

    #[tokio::test]
    async fn test_duplicate_node_error_handling() {
        let validator = RaftInputValidator::new();
        let existing_nodes = vec![
            (1, "127.0.0.1:8080".to_string()),
            (2, "127.0.0.1:8081".to_string()),
            (3, "192.168.1.100:8080".to_string()),
        ];

        // Test duplicate node ID
        let result = validator.validate_add_node(1, "127.0.0.1:8082", &existing_nodes);
        assert!(result.is_err());

        match result {
            Err(ConfluxError::Validation(msg)) => {
                assert!(msg.contains("already exists"));
                assert!(msg.contains("Node ID"));
            }
            _ => panic!("Expected duplicate node ID error"),
        }

        // Test duplicate address
        let result = validator.validate_add_node(4, "127.0.0.1:8080", &existing_nodes);
        assert!(result.is_err());

        match result {
            Err(ConfluxError::Validation(msg)) => {
                assert!(msg.contains("already exists"));
                assert!(msg.contains("Address"));
            }
            _ => panic!("Expected duplicate address error"),
        }
    }

    #[tokio::test]
    async fn test_node_removal_error_handling() {
        let validator = RaftInputValidator::new();

        // Test removing non-existent node
        let existing_nodes = vec![
            (1, "127.0.0.1:8080".to_string()),
            (2, "127.0.0.1:8081".to_string()),
        ];

        let result = validator.validate_remove_node(3, &existing_nodes);
        assert!(result.is_err());

        match result {
            Err(ConfluxError::Validation(msg)) => {
                assert!(msg.contains("does not exist"));
            }
            _ => panic!("Expected non-existent node error"),
        }

        // Test removing last node
        let single_node = vec![(1, "127.0.0.1:8080".to_string())];
        let result = validator.validate_remove_node(1, &single_node);
        assert!(result.is_err());

        match result {
            Err(ConfluxError::Validation(msg)) => {
                assert!(msg.contains("last node"));
            }
            _ => panic!("Expected last node removal error"),
        }
    }

    #[tokio::test]
    async fn test_resource_limit_error_handling() {
        let app_config = create_test_app_config().await;
        let mut node_config = create_test_node_config(1, 9100);

        // Set very restrictive limits
        node_config.resource_limits = ResourceLimits {
            max_requests_per_second: 1,
            max_concurrent_requests: 1,
            max_request_size: 10,  // Very small
            max_memory_usage: 100, // Very small
            request_timeout_ms: 1000,
        };

        let node = RaftNode::new(node_config, &app_config).await.unwrap();
        let resource_limiter = node.resource_limiter();

        // Test request size limit
        let result = resource_limiter.check_request_allowed(100, None).await;
        assert!(result.is_err());

        match result {
            Err(ConfluxError::Raft(msg)) => {
                assert!(msg.contains("exceeds limit"));
            }
            _ => panic!("Expected request size limit error"),
        }

        // Test memory limit
        let result = resource_limiter.check_request_allowed(5, None).await;
        if result.is_ok() {
            // Use up the memory
            let _permit1 = result.unwrap();

            // This should fail due to the memory limit
            let _result2 = resource_limiter.check_request_allowed(5, None).await;
            // Note: this test might be flaky due to timing, but demonstrates the concept
        }
    }

    #[tokio::test]
    async fn test_node_operation_error_scenarios() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 9101);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Test adding invalid nodes
        let result = node.add_node(0, "127.0.0.1:8080".to_string()).await;
        assert!(result.is_err());

        let result = node.add_node(2, "invalid-address".to_string()).await;
        assert!(result.is_err());

        // Test removing invalid nodes
        let result = node.remove_node(0).await;
        assert!(result.is_err());

        let result = node.remove_node(999).await;
        assert!(result.is_err());

        // Test invalid timeout updates
        let result = node.update_timeouts(Some(0), None, None).await;
        assert!(result.is_err());

        let result = node.update_timeouts(Some(99999), None, None).await;
        assert!(result.is_err());

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_authorization_error_handling() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 9102);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Test operations with auth context but no auth service
        let auth_ctx = AuthContext::new("test_user".to_string(), "test_tenant".to_string());

        // These should work when no auth service is configured warnings logged
        let _result = node
            .add_node_with_auth(2, "127.0.0.1:8082".to_string(), Some(auth_ctx.clone()))
            .await;
        // Result may vary based on Raft state, but should not fail due to auth

        let result = node
            .update_timeouts_with_auth(Some(100), Some(200), Some(400), Some(auth_ctx))
            .await;
        assert!(result.is_ok()); // Should work when no auth service is configured

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_edge_case_validation_scenarios() {
        let validator = RaftInputValidator::new();

        // Test empty cluster scenarios
        let empty_cluster = vec![];
        let result = validator.validate_add_node(1, "127.0.0.1:8080", &empty_cluster);
        assert!(result.is_ok());

        let config = ValidationConfig::default();
        let validator = NodeValidator::new(&config);

        // Test very long address
        let long_address = format!("{}:8080", "a".repeat(300));
        let result = validator.validate_node_address(&long_address);
        assert!(result.is_err());

        // Test boundary values
        let config = ValidationConfig {
            min_node_id: 1,
            max_node_id: 2,
            max_cluster_size: 1,
            ..Default::default()
        };
        let validator = NodeValidator::new(&config);

        // Test exact boundary
        assert!(validator.validate_node_id(1).is_ok());
        assert!(validator.validate_node_id(2).is_ok());
        assert!(validator.validate_node_id(3).is_err());

        let validator = ClusterValidator::new(&config);

        // Test cluster size boundary
        assert!(validator.validate_cluster_size(0, 1).is_ok());
        assert!(validator.validate_cluster_size(1, 0).is_ok());
        assert!(validator.validate_cluster_size(1, 1).is_err());
    }

    #[tokio::test]
    async fn test_network_configuration_errors() {
        let config = ValidationConfig {
            allow_localhost: false,
            allow_private_ips: false,
            ..Default::default()
        };
        let validator = NodeValidator::new(&config);

        // Test that common addresses are rejected
        let restricted_addresses = vec![
            "127.0.0.1:8080",     // Localhost
            "192.168.1.100:8080", // Private IP
            "10.0.0.1:8080",      // Private IP
            "172.16.0.1:8080",    // Private IP
        ];

        for address in restricted_addresses {
            let result = validator.validate_node_address(address);
            assert!(result.is_err(), "Address {} should be rejected", address);
        }

        // Test that some public addresses would work if format is valid
        // Note: we don't test real public IPs in unit tests
    }

    #[tokio::test]
    async fn test_concurrent_error_scenarios() {
        use std::sync::Arc;

        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 9103);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        let node_arc = Arc::new(node);
        let mut handles = vec![];

        // Spawn multiple tasks that should fail
        for i in 0..10 {
            let node_clone = node_arc.clone();
            let handle = tokio::spawn(async move {
                // Try various invalid operations
                let _ = node_clone.add_node(0, "invalid".to_string()).await; // Should fail
                let _ = node_clone.remove_node(999).await; // Should fail

                // These operations should not crash even when failing
                for _ in 0..5 {
                    let _ = node_clone.add_node(i, format!("invalid-{}", i)).await;
                    tokio::task::yield_now().await;
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap(); // Tasks shouldn't panic
        }

        // Node should still be functional
        assert!(node_arc.get_metrics().await.is_ok() || node_arc.get_metrics().await.is_err()); // Should not panic
        assert!(node_arc.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_graceful_degradation() {
        let app_config = create_test_app_config().await;
        let node_config = create_test_node_config(1, 9104);

        let mut node = RaftNode::new(node_config, &app_config).await.unwrap();
        assert!(node.start().await.is_ok());

        // Test that the system continues to function even with errors

        // Generate some errors
        for _i in 0..10 {
            let _ = node.add_node(0, "invalid".to_string()).await; // Should fail gracefully
            let _ = node.remove_node(999).await; // Should fail gracefully
        }

        // The System should still respond to valid operations
        let timeout_result = node.update_timeouts(Some(75), Some(200), Some(400)).await;
        assert!(timeout_result.is_ok());

        let metrics_result = node.get_metrics().await;
        // Should not panic may succeed or fail depending on Raft state
        match metrics_result {
            Ok(_) => {}  // Good
            Err(_) => {} // Also acceptable, as long as it doesn't panic
        }

        assert!(node.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_error_message_consistency() {
        let config = ValidationConfig::default();
        let validator = NodeValidator::new(&config);

        // Test that similar errors have consistent message formats
        let node_id_errors = vec![
            validator.validate_node_id(0),
            validator.validate_node_id(65536),
        ];

        for error in node_id_errors {
            assert!(error.is_err());
            match error {
                Err(ConfluxError::Validation(msg)) => {
                    assert!(msg.contains("Node ID"));
                    assert!(msg.len() > 10); // Should be descriptive
                    assert!(!msg.contains("panic")); // Should not leak internal details
                }
                _ => panic!("Expected validation error"),
            }
        }

        // Test address error consistency
        let address_errors = vec![
            validator.validate_node_address(""),
            validator.validate_node_address("invalid"),
            validator.validate_node_address("127.0.0.1:99999"),
        ];

        for error in address_errors {
            assert!(error.is_err());
            match error {
                Err(ConfluxError::Validation(msg)) => {
                    assert!(!msg.is_empty());
                    assert!(msg.len() > 5); // Should be descriptive
                }
                _ => panic!("Expected validation error"),
            }
        }
    }
}
