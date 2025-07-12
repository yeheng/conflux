#[cfg(test)]
mod raft_input_validator_tests {
    use crate::raft::{RaftInputValidator, ValidationConfig};

    #[test]
    fn test_raft_input_validator_creation() {
        let validator = RaftInputValidator::new();
        let config = validator.get_config();
        assert_eq!(config.min_node_id, 1);
        assert_eq!(config.max_node_id, 65535);
    }

    #[test]
    fn test_validate_add_node() {
        let validator = RaftInputValidator::new();
        let existing_nodes = vec![(1, "127.0.0.1:8080".to_string())];

        let result = validator.validate_add_node(2, "127.0.0.1:8081", &existing_nodes);
        assert!(result.is_ok());

        let result = validator.validate_add_node(1, "127.0.0.1:8082", &existing_nodes);
        assert!(result.is_err()); // Duplicate node ID
    }

    #[test]
    fn test_validate_remove_node() {
        let validator = RaftInputValidator::new();
        let existing_nodes = vec![
            (1, "127.0.0.1:8080".to_string()),
            (2, "127.0.0.1:8081".to_string()),
        ];

        let result = validator.validate_remove_node(2, &existing_nodes);
        assert!(result.is_ok());

        let result = validator.validate_remove_node(3, &existing_nodes);
        assert!(result.is_err()); // Non-existent node
    }

    #[test]
    fn test_validate_timeout_config() {
        let validator = RaftInputValidator::new();

        let result = validator.validate_timeout_config(Some(100), Some(300), Some(600));
        assert!(result.is_ok());

        let result = validator.validate_timeout_config(Some(0), None, None);
        assert!(result.is_err()); // Invalid heartbeat
    }

    #[test]
    fn test_with_custom_config() {
        let config = ValidationConfig::dev();
        let validator = RaftInputValidator::with_config(config);

        let validator_config = validator.get_config();
        assert_eq!(validator_config.max_cluster_size, 1000); // dev config has larger cluster size
    }

    #[test]
    fn test_cluster_suggestions() {
        let validator = RaftInputValidator::new();

        let suggestions = validator.get_cluster_suggestions(4, 100, 300, 10);
        assert!(suggestions.has_suggestions()); // Should suggest odd cluster size
    }
}
