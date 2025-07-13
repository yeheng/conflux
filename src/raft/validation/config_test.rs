#[cfg(test)]
mod confg_tests {
    use crate::raft::ValidationConfig;

    #[test]
    fn test_default_config() {
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
    fn test_dev_config() {
        let config = ValidationConfig::dev();
        assert!(config.allow_localhost);
        assert!(config.allow_private_ips);
        assert_eq!(config.max_cluster_size, 1000);
    }

    #[test]
    fn test_prod_config() {
        let config = ValidationConfig::prod();
        assert!(!config.allow_localhost);
        assert!(!config.allow_private_ips);
        assert_eq!(config.allowed_port_range, (8000, 9000));
    }

    #[test]
    fn test_config_validation() {
        let valid_config = ValidationConfig::default();
        assert!(valid_config.validate().is_ok());

        let invalid_config = ValidationConfig {
            min_node_id: 100,
            max_node_id: 50,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());

        let zero_min_config = ValidationConfig {
            min_node_id: 0,
            ..Default::default()
        };
        assert!(zero_min_config.validate().is_err());
    }

    #[test]
    fn test_config_setters() {
        let mut config = ValidationConfig::default();

        config.set_node_id_range(1, 1000);
        assert_eq!(config.min_node_id, 1);
        assert_eq!(config.max_node_id, 1000);

        config.set_port_range(8000, 9000);
        assert_eq!(config.allowed_port_range, (8000, 9000));

        config.set_network_policy(false, false);
        assert!(!config.allow_localhost);
        assert!(!config.allow_private_ips);

        config.set_max_cluster_size(50);
        assert_eq!(config.max_cluster_size, 50);
    }
}
