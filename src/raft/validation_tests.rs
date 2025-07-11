//! Comprehensive unit tests for input validation module

#[cfg(test)]
mod tests {
    use super::super::validation::{RaftInputValidator, ValidationConfig};
    use crate::error::ConfluxError;

    fn create_test_validator() -> RaftInputValidator {
        RaftInputValidator::new()
    }

    fn create_test_validator_with_config(config: ValidationConfig) -> RaftInputValidator {
        RaftInputValidator::with_config(config)
    }

    #[test]
    fn test_validation_config_default() {
        let config = ValidationConfig::default();
        assert_eq!(config.min_node_id, 1);
        assert_eq!(config.max_node_id, 65535);
        assert_eq!(config.allowed_port_range, (1024, 65535));
        assert_eq!(config.max_hostname_length, 253);
        assert!(config.allow_localhost);
        assert!(config.allow_private_ips);
        assert_eq!(config.max_cluster_size, 100);
    }

    #[test]
    fn test_validate_node_id_valid() {
        let validator = create_test_validator();
        assert!(validator.validate_node_id(1).is_ok());
        assert!(validator.validate_node_id(100).is_ok());
        assert!(validator.validate_node_id(65535).is_ok());
    }

    #[test]
    fn test_validate_node_id_invalid() {
        let validator = create_test_validator();
        assert!(validator.validate_node_id(0).is_err());
        assert!(validator.validate_node_id(65536).is_err());
    }

    #[test]
    fn test_validate_node_id_custom_range() {
        let config = ValidationConfig {
            min_node_id: 10,
            max_node_id: 100,
            ..Default::default()
        };
        let validator = create_test_validator_with_config(config);
        
        assert!(validator.validate_node_id(5).is_err());
        assert!(validator.validate_node_id(10).is_ok());
        assert!(validator.validate_node_id(50).is_ok());
        assert!(validator.validate_node_id(100).is_ok());
        assert!(validator.validate_node_id(101).is_err());
    }

    #[test]
    fn test_validate_node_address_valid() {
        let validator = create_test_validator();
        assert!(validator.validate_node_address("127.0.0.1:8080").is_ok());
        assert!(validator.validate_node_address("192.168.1.100:3000").is_ok());
        assert!(validator.validate_node_address("[::1]:8080").is_ok());
        assert!(validator.validate_node_address("10.0.0.1:9000").is_ok());
    }

    #[test]
    fn test_validate_node_address_invalid() {
        let validator = create_test_validator();
        assert!(validator.validate_node_address("").is_err());
        assert!(validator.validate_node_address("invalid").is_err());
        assert!(validator.validate_node_address("127.0.0.1:99999").is_err());
        assert!(validator.validate_node_address("127.0.0.1:80").is_err()); // Port too low
        assert!(validator.validate_node_address("256.256.256.256:8080").is_err()); // Invalid IP
    }

    #[test]
    fn test_validate_node_address_localhost_restrictions() {
        let config = ValidationConfig {
            allow_localhost: false,
            ..Default::default()
        };
        let validator = create_test_validator_with_config(config);
        
        assert!(validator.validate_node_address("127.0.0.1:8080").is_err());
        assert!(validator.validate_node_address("[::1]:8080").is_err());
        assert!(validator.validate_node_address("192.168.1.100:8080").is_ok()); // Private IP still allowed
    }

    #[test]
    fn test_validate_node_address_private_ip_restrictions() {
        let config = ValidationConfig {
            allow_private_ips: false,
            ..Default::default()
        };
        let validator = create_test_validator_with_config(config);
        
        assert!(validator.validate_node_address("192.168.1.100:8080").is_err());
        assert!(validator.validate_node_address("10.0.0.1:8080").is_err());
        assert!(validator.validate_node_address("172.16.0.1:8080").is_err());
        assert!(validator.validate_node_address("127.0.0.1:8080").is_ok()); // Localhost still allowed
    }

    #[test]
    fn test_validate_node_address_port_restrictions() {
        let config = ValidationConfig {
            allowed_port_range: (8000, 9000),
            ..Default::default()
        };
        let validator = create_test_validator_with_config(config);
        
        assert!(validator.validate_node_address("127.0.0.1:7999").is_err());
        assert!(validator.validate_node_address("127.0.0.1:8000").is_ok());
        assert!(validator.validate_node_address("127.0.0.1:8500").is_ok());
        assert!(validator.validate_node_address("127.0.0.1:9000").is_ok());
        assert!(validator.validate_node_address("127.0.0.1:9001").is_err());
    }

    #[test]
    fn test_validate_cluster_size_valid() {
        let validator = create_test_validator();
        assert!(validator.validate_cluster_size(5, 1).is_ok());
        assert!(validator.validate_cluster_size(99, 1).is_ok());
        assert!(validator.validate_cluster_size(0, 5).is_ok()); // Empty cluster growing
    }

    #[test]
    fn test_validate_cluster_size_invalid() {
        let validator = create_test_validator();
        assert!(validator.validate_cluster_size(100, 1).is_err()); // Would exceed max
        assert!(validator.validate_cluster_size(50, 51).is_err()); // Adding too many
    }

    #[test]
    fn test_validate_cluster_size_custom_limit() {
        let config = ValidationConfig {
            max_cluster_size: 10,
            ..Default::default()
        };
        let validator = create_test_validator_with_config(config);
        
        assert!(validator.validate_cluster_size(9, 1).is_ok());
        assert!(validator.validate_cluster_size(10, 1).is_err());
        assert!(validator.validate_cluster_size(5, 6).is_err());
    }

    #[test]
    fn test_validate_node_id_uniqueness() {
        let validator = create_test_validator();
        let existing_nodes = vec![1, 2, 3, 5];
        
        assert!(validator.validate_node_id_uniqueness(4, &existing_nodes).is_ok());
        assert!(validator.validate_node_id_uniqueness(6, &existing_nodes).is_ok());
        assert!(validator.validate_node_id_uniqueness(2, &existing_nodes).is_err()); // Duplicate
        assert!(validator.validate_node_id_uniqueness(1, &existing_nodes).is_err()); // Duplicate
    }

    #[test]
    fn test_validate_address_uniqueness() {
        let validator = create_test_validator();
        let existing_addresses = vec![
            "127.0.0.1:8080".to_string(),
            "192.168.1.100:9000".to_string(),
        ];
        
        assert!(validator.validate_address_uniqueness("127.0.0.1:8081", &existing_addresses).is_ok());
        assert!(validator.validate_address_uniqueness("192.168.1.101:9000", &existing_addresses).is_ok());
        assert!(validator.validate_address_uniqueness("127.0.0.1:8080", &existing_addresses).is_err()); // Duplicate
        assert!(validator.validate_address_uniqueness("192.168.1.100:9000", &existing_addresses).is_err()); // Duplicate
    }

    #[test]
    fn test_validate_timeout_config_valid() {
        let validator = create_test_validator();
        
        // Valid individual timeouts
        assert!(validator.validate_timeout_config(Some(100), None, None).is_ok());
        assert!(validator.validate_timeout_config(None, Some(300), None).is_ok());
        assert!(validator.validate_timeout_config(None, None, Some(600)).is_ok());
        
        // Valid combinations
        assert!(validator.validate_timeout_config(Some(100), Some(300), Some(600)).is_ok());
        assert!(validator.validate_timeout_config(Some(150), Some(300), None).is_ok());
    }

    #[test]
    fn test_validate_timeout_config_invalid() {
        let validator = create_test_validator();
        
        // Zero values
        assert!(validator.validate_timeout_config(Some(0), None, None).is_err());
        assert!(validator.validate_timeout_config(None, Some(0), None).is_err());
        assert!(validator.validate_timeout_config(None, None, Some(0)).is_err());
        
        // Too large values
        assert!(validator.validate_timeout_config(Some(15000), None, None).is_err());
        assert!(validator.validate_timeout_config(None, Some(50000), None).is_err());
        assert!(validator.validate_timeout_config(None, None, Some(70000)).is_err());
        
        // Invalid relationships
        assert!(validator.validate_timeout_config(Some(500), Some(300), None).is_err()); // heartbeat >= min
        assert!(validator.validate_timeout_config(Some(600), None, Some(300)).is_err()); // heartbeat >= max
        assert!(validator.validate_timeout_config(None, Some(600), Some(300)).is_err()); // min >= max
    }

    #[test]
    fn test_validate_add_node_comprehensive() {
        let validator = create_test_validator();
        let existing_nodes = vec![
            (1, "127.0.0.1:8080".to_string()),
            (2, "127.0.0.1:8081".to_string()),
        ];
        
        // Valid addition
        assert!(validator.validate_add_node(3, "127.0.0.1:8082", &existing_nodes).is_ok());
        
        // Invalid additions
        assert!(validator.validate_add_node(1, "127.0.0.1:8082", &existing_nodes).is_err()); // Duplicate ID
        assert!(validator.validate_add_node(3, "127.0.0.1:8080", &existing_nodes).is_err()); // Duplicate address
        assert!(validator.validate_add_node(0, "127.0.0.1:8082", &existing_nodes).is_err()); // Invalid ID
        assert!(validator.validate_add_node(3, "invalid-address", &existing_nodes).is_err()); // Invalid address
    }

    #[test]
    fn test_validate_add_node_cluster_size_limit() {
        let config = ValidationConfig {
            max_cluster_size: 3,
            ..Default::default()
        };
        let validator = create_test_validator_with_config(config);
        let existing_nodes = vec![
            (1, "127.0.0.1:8080".to_string()),
            (2, "127.0.0.1:8081".to_string()),
            (3, "127.0.0.1:8082".to_string()),
        ];
        
        // Should fail due to cluster size limit
        assert!(validator.validate_add_node(4, "127.0.0.1:8083", &existing_nodes).is_err());
    }

    #[test]
    fn test_validate_remove_node_valid() {
        let validator = create_test_validator();
        let existing_nodes = vec![
            (1, "127.0.0.1:8080".to_string()),
            (2, "127.0.0.1:8081".to_string()),
        ];
        
        assert!(validator.validate_remove_node(1, &existing_nodes).is_ok());
        assert!(validator.validate_remove_node(2, &existing_nodes).is_ok());
    }

    #[test]
    fn test_validate_remove_node_invalid() {
        let validator = create_test_validator();
        let existing_nodes = vec![
            (1, "127.0.0.1:8080".to_string()),
            (2, "127.0.0.1:8081".to_string()),
        ];
        
        // Non-existent node
        assert!(validator.validate_remove_node(3, &existing_nodes).is_err());
        
        // Cannot remove last node
        let single_node = vec![(1, "127.0.0.1:8080".to_string())];
        assert!(validator.validate_remove_node(1, &single_node).is_err());
    }

    #[test]
    fn test_validate_remove_node_invalid_id() {
        let validator = create_test_validator();
        let existing_nodes = vec![
            (1, "127.0.0.1:8080".to_string()),
            (2, "127.0.0.1:8081".to_string()),
        ];
        
        // Invalid node ID (outside allowed range)
        assert!(validator.validate_remove_node(0, &existing_nodes).is_err());
        assert!(validator.validate_remove_node(65536, &existing_nodes).is_err());
    }

    #[test]
    fn test_update_validation_config() {
        let mut validator = create_test_validator();
        
        // Original config allows node ID 1
        assert!(validator.validate_node_id(1).is_ok());
        
        // Update config to disallow node ID 1
        let new_config = ValidationConfig {
            min_node_id: 10,
            ..Default::default()
        };
        validator.update_config(new_config);
        
        // Now node ID 1 should be invalid
        assert!(validator.validate_node_id(1).is_err());
        assert!(validator.validate_node_id(10).is_ok());
    }

    #[test]
    fn test_get_validation_config() {
        let config = ValidationConfig {
            min_node_id: 5,
            max_node_id: 1000,
            ..Default::default()
        };
        let validator = create_test_validator_with_config(config.clone());
        
        let retrieved_config = validator.get_config();
        assert_eq!(retrieved_config.min_node_id, 5);
        assert_eq!(retrieved_config.max_node_id, 1000);
    }

    #[test]
    fn test_ipv6_address_validation() {
        let validator = create_test_validator();
        
        // Valid IPv6 addresses
        assert!(validator.validate_node_address("[::1]:8080").is_ok());
        assert!(validator.validate_node_address("[2001:db8::1]:9000").is_ok());
        
        // Invalid IPv6 addresses
        assert!(validator.validate_node_address("[::]:8080").is_err()); // Unspecified
        assert!(validator.validate_node_address("[ff02::1]:8080").is_err()); // Multicast
    }

    #[test]
    fn test_ipv6_private_address_restrictions() {
        let config = ValidationConfig {
            allow_private_ips: false,
            ..Default::default()
        };
        let validator = create_test_validator_with_config(config);
        
        // Should reject private IPv6 addresses
        assert!(validator.validate_node_address("[fc00::1]:8080").is_err()); // Unique local
        assert!(validator.validate_node_address("[fe80::1]:8080").is_err()); // Link-local
    }

    #[test]
    fn test_edge_case_cluster_sizes() {
        let validator = create_test_validator();
        
        // Edge case: empty cluster
        assert!(validator.validate_cluster_size(0, 1).is_ok());
        
        // Edge case: single node cluster
        assert!(validator.validate_cluster_size(1, 0).is_ok());
        
        // Edge case: exactly at limit
        assert!(validator.validate_cluster_size(99, 1).is_ok());
        assert!(validator.validate_cluster_size(100, 0).is_ok());
    }

    #[test]
    fn test_error_message_quality() {
        let validator = create_test_validator();
        
        // Test that error messages contain useful information
        match validator.validate_node_id(0) {
            Err(ConfluxError::Validation(msg)) => {
                assert!(msg.contains("Node ID"));
                assert!(msg.contains("minimum"));
            },
            _ => panic!("Expected validation error"),
        }
        
        match validator.validate_node_address("invalid") {
            Err(ConfluxError::Validation(msg)) => {
                assert!(msg.contains("socket address"));
            },
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_concurrent_validation_safety() {
        use std::sync::Arc;
        use std::thread;
        
        let validator = Arc::new(create_test_validator());
        let mut handles = vec![];
        
        // Spawn multiple threads doing validation
        for i in 1..=10 {
            let validator_clone = validator.clone();
            let handle = thread::spawn(move || {
                let result = validator_clone.validate_node_id(i);
                assert!(result.is_ok());
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }
}