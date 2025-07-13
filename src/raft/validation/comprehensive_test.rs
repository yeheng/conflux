#[cfg(test)]
mod comprehensive_tests {
    use crate::raft::validation::*;

    #[test]
    fn test_validate_add_node() {
        let config = ValidationConfig::default();
        let validator = ComprehensiveValidator::new(config);
        let existing_nodes = vec![(1, "127.0.0.1:8080".to_string())];

        // Valid addition
        let result = validator.validate_add_node(2, "127.0.0.1:8081", &existing_nodes);
        assert!(result.is_ok());

        // Duplicate node ID
        let result = validator.validate_add_node(1, "127.0.0.1:8082", &existing_nodes);
        assert!(result.is_err());

        // Duplicate address
        let result = validator.validate_add_node(3, "127.0.0.1:8080", &existing_nodes);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_remove_node() {
        let config = ValidationConfig::default();
        let validator = ComprehensiveValidator::new(config);
        let existing_nodes = vec![
            (1, "127.0.0.1:8080".to_string()),
            (2, "127.0.0.1:8081".to_string()),
        ];

        // Valid removal
        let result = validator.validate_remove_node(2, &existing_nodes);
        assert!(result.is_ok());

        // Non-existent node
        let result = validator.validate_remove_node(3, &existing_nodes);
        assert!(result.is_err());

        // Cannot remove last node
        let single_node = vec![(1, "127.0.0.1:8080".to_string())];
        let result = validator.validate_remove_node(1, &single_node);
        assert!(result.is_err());
    }

    #[test]
    fn test_cluster_suggestions() {
        let config = ValidationConfig::default();
        let validator = ComprehensiveValidator::new(config);

        let suggestions = validator.get_cluster_suggestions(4, 100, 300, 10);

        // Should suggest odd cluster size
        assert!(suggestions.has_suggestions());
        assert!(suggestions
            .size_recommendations
            .iter()
            .any(|s| s.contains("odd cluster size")));
    }

    #[test]
    fn test_update_config() {
        let initial_config = ValidationConfig::default();
        let mut validator = ComprehensiveValidator::new(initial_config);
        assert_eq!(validator.get_config().max_cluster_size, 100);

        let mut new_config = ValidationConfig::default();
        new_config.max_cluster_size = 200;

        validator.update_config(new_config);
        assert_eq!(validator.get_config().max_cluster_size, 200);
        // Also check if sub-validators got the new config
        assert_eq!(validator.cluster_validator.config().max_cluster_size, 200);
    }
}
