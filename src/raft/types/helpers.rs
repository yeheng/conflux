use super::config::ConfigNamespace;

/// Configuration key type for internal storage
pub type ConfigKey = String;

/// Helper function to create config key
pub fn make_config_key(namespace: &ConfigNamespace, name: &str) -> ConfigKey {
    format!("{}/{}", namespace, name)
}

/// Helper function to create config ID key
pub fn make_config_id_key(config_id: u64) -> Vec<u8> {
    let mut key = vec![0x02];
    key.extend_from_slice(&config_id.to_be_bytes());
    key
}

/// Helper function to create version key
pub fn make_version_key(config_id: u64, version_id: u64) -> Vec<u8> {
    let mut key = vec![0x03];
    key.extend_from_slice(&config_id.to_be_bytes());
    key.extend_from_slice(&version_id.to_be_bytes());
    key
}

/// Helper function to create name index key
pub fn make_name_index_key(namespace: &ConfigNamespace, name: &str) -> Vec<u8> {
    let mut key = vec![0x04];
    let name_key = format!("{}/{}", namespace, name);
    key.extend_from_slice(name_key.as_bytes());
    key
}

/// Helper function to create reverse index key
pub fn make_reverse_index_key(config_id: u64) -> Vec<u8> {
    let mut key = vec![0x05];
    key.extend_from_slice(&config_id.to_be_bytes());
    key
}
