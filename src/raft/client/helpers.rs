use crate::raft::types::*;
use super::types::*;
use std::collections::BTreeMap;

/// Helper function to create a write request
pub fn create_write_request(command: RaftCommand) -> ClientWriteRequest {
    ClientWriteRequest {
        command,
        request_id: None,
    }
}

/// Helper function to create a read request
pub fn create_read_request(operation: ReadOperation) -> ClientReadRequest {
    ClientReadRequest {
        operation,
        consistency: Some(ReadConsistency::default()),
    }
}

/// Helper function to create a get config request
pub fn create_get_config_request(
    namespace: ConfigNamespace,
    name: String,
    client_labels: BTreeMap<String, String>,
) -> ClientReadRequest {
    create_read_request(ReadOperation::GetConfig {
        namespace,
        name,
        client_labels,
    })
}

/// Helper function to create a list configs request
pub fn create_list_configs_request(
    namespace: ConfigNamespace,
    prefix: Option<String>,
) -> ClientReadRequest {
    create_read_request(ReadOperation::ListConfigs { namespace, prefix })
}
