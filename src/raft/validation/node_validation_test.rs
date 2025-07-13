#[cfg(test)]
mod node_validation_tests {
    use crate::raft::{validation::NodeValidator, ValidationConfig};

    use std::sync::Arc;

    #[test]
    fn test_validate_node_id() {
        let config = ValidationConfig::default();
        let validator = NodeValidator::new(Arc::new(config));

        // Valid node IDs
        assert!(validator.validate_node_id(1).is_ok());
        assert!(validator.validate_node_id(100).is_ok());
        assert!(validator.validate_node_id(65535).is_ok());

        // Invalid node IDs
        assert!(validator.validate_node_id(0).is_err());
        assert!(validator.validate_node_id(65536).is_err());
    }

    #[test]
    fn test_validate_node_address() {
        let config = ValidationConfig::default();
        let validator = NodeValidator::new(Arc::new(config));

        // Valid addresses
        assert!(validator.validate_node_address("127.0.0.1:8080").is_ok());
        assert!(validator
            .validate_node_address("192.168.1.100:3000")
            .is_ok());
        assert!(validator.validate_node_address("[::1]:8080").is_ok());

        // Invalid addresses
        assert!(validator.validate_node_address("").is_err());
        assert!(validator.validate_node_address("invalid").is_err());
        assert!(validator.validate_node_address("127.0.0.1:99999").is_err());
        assert!(validator.validate_node_address("127.0.0.1:80").is_err()); // Port too low
    }

    #[test]
    fn test_strict_network_policy() {
        let mut config = ValidationConfig::default();
        config.allow_localhost = false;
        config.allow_private_ips = false;

        let validator = NodeValidator::new(Arc::new(config));

        // These should fail with strict policy
        assert!(validator.validate_node_address("127.0.0.1:8080").is_err());
        assert!(validator.validate_node_address("192.168.1.1:8080").is_err());
        assert!(validator.validate_node_address("10.0.0.1:8080").is_err());
    }
}
