#[cfg(test)]
mod tests {
    use crate::raft::types::{ConfigNamespace, ConfigFormat, Release, RaftCommand, ClientRequest, ClientWriteResponse};
    use std::collections::BTreeMap;

    #[test]
    fn test_raft_command_create_config() {
        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "myapp".to_string(),
            env: "dev".to_string(),
        };

        let command = RaftCommand::CreateConfig {
            namespace: namespace.clone(),
            name: "test.json".to_string(),
            content: b"{}".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Test config".to_string(),
        };

        assert_eq!(command.config_id(), None);
        assert_eq!(command.creator_id(), Some(1));
        assert!(command.modifies_content());
        assert!(!command.modifies_releases());
    }

    #[test]
    fn test_raft_command_create_version() {
        let command = RaftCommand::CreateVersion {
            config_id: 123,
            content: b"new content".to_vec(),
            format: Some(ConfigFormat::Yaml),
            creator_id: 2,
            description: "New version".to_string(),
        };

        assert_eq!(command.config_id(), Some(123));
        assert_eq!(command.creator_id(), Some(2));
        assert!(command.modifies_content());
        assert!(!command.modifies_releases());
    }

    #[test]
    fn test_raft_command_update_release_rules() {
        let mut labels = BTreeMap::new();
        labels.insert("env".to_string(), "prod".to_string());
        
        let releases = vec![Release::new(labels, 1, 10)];
        
        let command = RaftCommand::UpdateReleaseRules {
            config_id: 456,
            releases,
        };

        assert_eq!(command.config_id(), Some(456));
        assert_eq!(command.creator_id(), None);
        assert!(!command.modifies_content());
        assert!(command.modifies_releases());
    }

    #[test]
    fn test_raft_command_delete_config() {
        let command = RaftCommand::DeleteConfig { config_id: 789 };

        assert_eq!(command.config_id(), Some(789));
        assert_eq!(command.creator_id(), None);
        assert!(!command.modifies_content());
        assert!(!command.modifies_releases());
    }

    #[test]
    fn test_raft_command_delete_versions() {
        let command = RaftCommand::DeleteVersions {
            config_id: 101,
            version_ids: vec![1, 2, 3],
        };

        assert_eq!(command.config_id(), Some(101));
        assert_eq!(command.creator_id(), None);
        assert!(!command.modifies_content());
        assert!(!command.modifies_releases());
    }

    #[test]
    fn test_raft_command_release_version() {
        let command = RaftCommand::ReleaseVersion {
            config_id: 202,
            version_id: 5,
        };

        assert_eq!(command.config_id(), Some(202));
        assert_eq!(command.creator_id(), None);
        assert!(!command.modifies_content());
        assert!(command.modifies_releases());
    }

    #[test]
    fn test_raft_command_update_config() {
        let namespace = ConfigNamespace {
            tenant: "test".to_string(),
            app: "myapp".to_string(),
            env: "prod".to_string(),
        };

        let command = RaftCommand::UpdateConfig {
            config_id: 303,
            namespace,
            name: "updated.toml".to_string(),
            content: b"updated content".to_vec(),
            format: ConfigFormat::Toml,
            schema: Some("v2".to_string()),
            description: "Updated config".to_string(),
        };

        assert_eq!(command.config_id(), Some(303));
        assert_eq!(command.creator_id(), None);
        assert!(command.modifies_content());
        assert!(!command.modifies_releases());
    }

    #[test]
    fn test_client_request_serialization() {
        let command = RaftCommand::CreateConfig {
            namespace: ConfigNamespace {
                tenant: "test".to_string(),
                app: "app".to_string(),
                env: "dev".to_string(),
            },
            name: "config".to_string(),
            content: b"test".to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "test".to_string(),
        };

        let request = ClientRequest { command };
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: ClientRequest = serde_json::from_str(&serialized).unwrap();

        match (&request.command, &deserialized.command) {
            (RaftCommand::CreateConfig { name: n1, .. }, RaftCommand::CreateConfig { name: n2, .. }) => {
                assert_eq!(n1, n2);
            }
            _ => panic!("Deserialization failed"),
        }
    }

    #[test]
    fn test_client_write_response_default() {
        let response = ClientWriteResponse::default();
        assert_eq!(response.config_id, None);
        assert!(!response.success);
        assert_eq!(response.message, "No operation performed");
        assert_eq!(response.data, None);
    }

    #[test]
    fn test_client_write_response_success() {
        let response = ClientWriteResponse {
            config_id: Some(123),
            success: true,
            message: "Operation successful".to_string(),
            data: Some(serde_json::json!({"version": 1})),
        };

        assert_eq!(response.config_id, Some(123));
        assert!(response.success);
        assert_eq!(response.message, "Operation successful");
        assert!(response.data.is_some());
    }

    #[test]
    fn test_client_write_response_serialization() {
        let response = ClientWriteResponse {
            config_id: Some(456),
            success: true,
            message: "Test message".to_string(),
            data: Some(serde_json::json!({"key": "value"})),
        };

        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: ClientWriteResponse = serde_json::from_str(&serialized).unwrap();

        assert_eq!(response.config_id, deserialized.config_id);
        assert_eq!(response.success, deserialized.success);
        assert_eq!(response.message, deserialized.message);
        assert_eq!(response.data, deserialized.data);
    }
}
