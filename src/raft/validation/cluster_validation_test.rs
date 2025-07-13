#[cfg(test)]
mod cluster_validation_tests {
    use crate::raft::validation::*;
    use std::sync::Arc;

    #[test]
    fn test_validate_cluster_size() {
        let config = ValidationConfig::default();
        let validator = ClusterValidator::new(Arc::new(config));

        // Valid cluster sizes
        assert!(validator.validate_cluster_size(5, 1).is_ok());
        assert!(validator.validate_cluster_size(99, 1).is_ok());

        // Invalid cluster sizes
        assert!(validator.validate_cluster_size(100, 1).is_err());
        assert!(validator.validate_cluster_size(150, 0).is_err());
    }

    #[test]
    fn test_validate_minimum_cluster_size() {
        let config = ValidationConfig::default();
        let validator = ClusterValidator::new(Arc::new(config));

        // Valid removals
        assert!(validator.validate_minimum_cluster_size(3, 1).is_ok());
        assert!(validator.validate_minimum_cluster_size(5, 2).is_ok());

        // Invalid removals
        assert!(validator.validate_minimum_cluster_size(1, 1).is_err());
        assert!(validator.validate_minimum_cluster_size(3, 3).is_err());
        assert!(validator.validate_minimum_cluster_size(2, 3).is_err());
    }

    #[test]
    fn test_validate_cluster_parity() {
        let config = ValidationConfig::default();
        let validator = ClusterValidator::new(Arc::new(config));

        // Odd sizes (recommended)
        assert!(validator.validate_cluster_parity(1));
        assert!(validator.validate_cluster_parity(3));
        assert!(validator.validate_cluster_parity(5));

        // Even sizes (not recommended but allowed)
        assert!(!validator.validate_cluster_parity(2));
        assert!(!validator.validate_cluster_parity(4));
        assert!(!validator.validate_cluster_parity(6));
    }

    #[test]
    fn test_validate_cluster_health() {
        let config = ValidationConfig::default();
        let validator = ClusterValidator::new(Arc::new(config));

        // Healthy clusters
        assert!(validator.validate_cluster_health(5, 3).is_ok()); // Majority healthy
        assert!(validator.validate_cluster_health(3, 2).is_ok()); // Majority healthy
        assert!(validator.validate_cluster_health(1, 1).is_ok()); // Single node

        // Unhealthy clusters
        assert!(validator.validate_cluster_health(5, 2).is_err()); // Minority healthy
        assert!(validator.validate_cluster_health(3, 1).is_err()); // Minority healthy
        assert!(validator.validate_cluster_health(5, 0).is_err()); // No healthy nodes
        assert!(validator.validate_cluster_health(0, 0).is_err()); // Empty cluster
    }

    #[test]
    fn test_fault_tolerance_calculation() {
        let config = ValidationConfig::default();
        let validator = ClusterValidator::new(Arc::new(config));

        assert_eq!(validator.calculate_fault_tolerance(1), 0);
        assert_eq!(validator.calculate_fault_tolerance(3), 1);
        assert_eq!(validator.calculate_fault_tolerance(5), 2);
        assert_eq!(validator.calculate_fault_tolerance(7), 3);
    }

    #[test]
    fn test_recommend_cluster_size() {
        let config = ValidationConfig::default();
        let validator = ClusterValidator::new(Arc::new(config));

        assert_eq!(validator.recommend_cluster_size(0), 1);
        assert_eq!(validator.recommend_cluster_size(1), 3);
        assert_eq!(validator.recommend_cluster_size(2), 5);
        assert_eq!(validator.recommend_cluster_size(3), 7);
    }

    #[test]
    fn test_validate_node_exists() {
        let config = ValidationConfig::default();
        let validator = ClusterValidator::new(Arc::new(config));
        let existing_nodes = vec![
            (1, "127.0.0.1:8080".to_string()),
            (2, "127.0.0.1:8081".to_string()),
        ];

        // Existing nodes
        assert!(validator.validate_node_exists(1, &existing_nodes).is_ok());
        assert!(validator.validate_node_exists(2, &existing_nodes).is_ok());

        // Non-existing node
        assert!(validator.validate_node_exists(3, &existing_nodes).is_err());
    }
}
