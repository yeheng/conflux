#[cfg(test)]
mod timeouut_validation_tests {
    use crate::raft::validation::*;

    #[test]
    fn test_validate_timeout_config() {
        let validator = TimeoutValidator::new();

        // Valid timeouts
        assert!(validator
            .validate_timeout_config(Some(100), Some(300), Some(600))
            .is_ok());
        assert!(validator
            .validate_timeout_config(Some(50), None, None)
            .is_ok());

        // Invalid timeouts
        assert!(validator
            .validate_timeout_config(Some(0), None, None)
            .is_err());
        assert!(validator
            .validate_timeout_config(Some(500), Some(300), None)
            .is_err()); // heartbeat >= min
        assert!(validator
            .validate_timeout_config(None, Some(600), Some(300))
            .is_err()); // min >= max
    }

    #[test]
    fn test_validate_heartbeat_interval() {
        let validator = TimeoutValidator::new();

        // Valid intervals
        assert!(validator.validate_heartbeat_interval(50).is_ok());
        assert!(validator.validate_heartbeat_interval(100).is_ok());
        assert!(validator.validate_heartbeat_interval(1000).is_ok());

        // Invalid intervals
        assert!(validator.validate_heartbeat_interval(0).is_err());
        assert!(validator.validate_heartbeat_interval(5).is_err());
        assert!(validator.validate_heartbeat_interval(15000).is_err());
    }

    #[test]
    fn test_validate_election_timeouts() {
        let validator = TimeoutValidator::new();

        // Valid timeouts
        assert!(validator.validate_election_timeout_min(100).is_ok());
        assert!(validator.validate_election_timeout_max(200).is_ok());

        // Invalid timeouts
        assert!(validator.validate_election_timeout_min(0).is_err());
        assert!(validator.validate_election_timeout_min(30).is_err());
        assert!(validator.validate_election_timeout_max(70000).is_err());
    }

    #[test]
    fn test_recommend_timeouts() {
        let validator = TimeoutValidator::new();

        let (heartbeat, min_timeout, max_timeout) = validator.recommend_timeouts(10);

        assert!(heartbeat >= 50); // Minimum heartbeat
        assert!(heartbeat < min_timeout);
        assert!(min_timeout < max_timeout);
        assert_eq!(max_timeout, min_timeout * 2);
    }

    #[test]
    fn test_validate_for_network() {
        let validator = TimeoutValidator::new();

        // Good configuration for 10ms latency
        assert!(validator.validate_for_network(50, 300, 10).is_ok());

        // Bad configuration - heartbeat too small
        assert!(validator.validate_for_network(15, 300, 10).is_err());

        // Bad configuration - election timeout too small
        assert!(validator.validate_for_network(50, 100, 10).is_err());
    }
}
