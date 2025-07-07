#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::raft::client::helpers::{create_write_request, create_get_config_request};
    use crate::raft::store::Store;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_client_write() {
        use tempfile::tempdir;
        let temp_dir = tempdir().unwrap();
        let store = Arc::new(Store::new(temp_dir.path()).await.unwrap());
        let client = RaftClient::new(store);

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
        let response = client.write(request).await.unwrap();

        assert!(response.success);
    }

    #[tokio::test]
    async fn test_client_read() {
        use tempfile::tempdir;
        let temp_dir = tempdir().unwrap();
        let store = Arc::new(Store::new(temp_dir.path()).await.unwrap());
        let client = RaftClient::new(store);

        let request = create_get_config_request(
            ConfigNamespace {
                tenant: "test".to_string(),
                app: "app".to_string(),
                env: "dev".to_string(),
            },
            "test-config".to_string(),
            BTreeMap::new(),
        );

        let response = client.read(request).await.unwrap();
        assert!(response.success);
    }

    #[tokio::test]
    async fn test_cluster_status() {
        use tempfile::tempdir;
        let temp_dir = tempdir().unwrap();
        let store = Arc::new(Store::new(temp_dir.path()).await.unwrap());
        let client = RaftClient::new(store);

        let status = client.get_cluster_status().await.unwrap();
        assert_eq!(status.members, vec![1]);
    }
}
