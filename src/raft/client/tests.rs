#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::raft::client::helpers::{create_write_request, create_get_config_request};
    use crate::raft::store::Store;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    async fn create_test_client() -> (RaftClient, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let (store, _) = Store::new(temp_dir.path()).await.unwrap();
        let client = RaftClient::new(Arc::new(store));
        (client, temp_dir)
    }

    #[tokio::test]
    async fn test_client_write() {
        let (client, _temp_dir) = create_test_client().await;

        let command = RaftCommand::CreateConfig {
            namespace: ConfigNamespace {
                tenant: "test".to_string(),
                app: "app".to_string(),
                env: "dev".to_string(),
            },
            name: "test-config".to_string(),
            content: "test content".as_bytes().to_vec(),
            format: ConfigFormat::Json,
            schema: None,
            creator_id: 1,
            description: "Test configuration".to_string(),
        };

        let request = create_write_request(command);
        // Client without Raft node should return error
        let result = client.write(request).await;
        assert!(result.is_err());

        // Verify the error message
        match result {
            Err(crate::error::ConfluxError::Raft(msg)) => {
                assert!(msg.contains("No Raft node available"));
            }
            _ => panic!("Expected Raft error"),
        }
    }

    #[tokio::test]
    async fn test_client_read() {
        let (client, _temp_dir) = create_test_client().await;

        let request = create_get_config_request(
            ConfigNamespace {
                tenant: "test".to_string(),
                app: "app".to_string(),
                env: "dev".to_string(),
            },
            "test-config".to_string(),
            BTreeMap::new(),
        );

        // Client without Raft node should return error
        let result = client.read(request).await;
        assert!(result.is_err());

        // Verify the error message
        match result {
            Err(crate::error::ConfluxError::Raft(msg)) => {
                assert!(msg.contains("No Raft node available"));
            }
            _ => panic!("Expected Raft error"),
        }
    }

    #[tokio::test]
    async fn test_cluster_status() {
        let (client, _temp_dir) = create_test_client().await;

        let status = client.get_cluster_status().await.unwrap();
        // The client returns a hardcoded status for MVP
        // According to the implementation, it returns vec![1] as members
        assert_eq!(status.members, vec![1]);
        assert_eq!(status.leader_id, Some(1));
        assert_eq!(status.term, 1);
    }
    
}
