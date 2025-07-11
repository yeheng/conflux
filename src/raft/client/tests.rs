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
        let response = client.write(request).await.unwrap();

        assert!(response.success);
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

        let response = client.read(request).await.unwrap();
        assert!(response.success);
    }

    #[tokio::test]
    async fn test_cluster_status() {
        let (client, _temp_dir) = create_test_client().await;

        let status = client.get_cluster_status().await.unwrap();
        // The default node id is 1, so the members should contain 1.
        // The actual members will depend on the default membership config.
        // Let's check the default.
        // StoredMembership::default() has an empty membership.
        // When the first node starts, it should add itself to the membership.
        // This logic is in the RaftNode.
        // The client just reads the state from the store.
        // The initial state is empty.
        // So the members should be empty.
        // Let's check the implementation of get_cluster_status.
        // It reads `last_membership` from the state machine.
        // Initially, this is empty.
        // So the test should assert for an empty vec.
        // The original test asserted `vec![1]`. This is probably wrong.
        // Let's check the RaftNode initialization.
        // `RaftNode::new` can initialize the cluster.
        // `self.raft.initialize(btreeset! {self.id}).await?;`
        // This will add the node to the cluster.
        // The client tests don't start a RaftNode, they just test the client against the store.
        // So the membership will be empty.
        assert!(status.members.is_empty());
    }
    
}
