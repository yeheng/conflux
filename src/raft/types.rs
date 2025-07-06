use openraft::{BasicNode, Raft};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Node ID type for the Raft cluster
pub type NodeId = u64;

/// Node information for cluster membership
pub type Node = BasicNode;

/// Configuration namespace identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ConfigNamespace {
    pub tenant: String,
    pub app: String,
    pub env: String,
}

impl std::fmt::Display for ConfigNamespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.tenant, self.app, self.env)
    }
}

/// Configuration format enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Yaml,
    Toml,
    Properties,
    Xml,
}

/// Core configuration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub id: u64,
    pub namespace: ConfigNamespace,
    pub name: String,
    pub latest_version_id: u64,
    pub releases: Vec<Release>,
    pub schema: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Config {
    /// Create a name key for indexing
    pub fn name_key(&self) -> String {
        make_config_key(&self.namespace, &self.name)
    }

    /// Get the default release (highest priority or fallback)
    pub fn get_default_release(&self) -> Option<&Release> {
        self.releases.iter().max_by_key(|r| r.priority)
    }

    /// Find matching release for given client labels
    pub fn find_matching_release(
        &self,
        client_labels: &BTreeMap<String, String>,
    ) -> Option<&Release> {
        let mut matching_releases: Vec<_> = self
            .releases
            .iter()
            .filter(|release| {
                // Check if client labels match release labels
                release
                    .labels
                    .iter()
                    .all(|(key, value)| client_labels.get(key) == Some(value))
            })
            .collect();

        // Sort by priority (descending)
        matching_releases.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Return the highest priority matching release
        matching_releases.first().copied()
    }
}

/// Immutable configuration version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigVersion {
    pub id: u64,
    pub config_id: u64,
    pub content: Vec<u8>,
    pub content_hash: String,
    pub format: ConfigFormat,
    pub creator_id: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub description: String,
}

impl ConfigVersion {
    /// Create a new ConfigVersion with computed hash
    pub fn new(
        id: u64,
        config_id: u64,
        content: Vec<u8>,
        format: ConfigFormat,
        creator_id: u64,
        description: String,
    ) -> Self {
        use sha2::{Digest, Sha256};
        let content_hash = format!("{:x}", Sha256::digest(&content));

        Self {
            id,
            config_id,
            content,
            content_hash,
            format,
            creator_id,
            created_at: chrono::Utc::now(),
            description,
        }
    }

    /// Verify content integrity
    pub fn verify_integrity(&self) -> bool {
        use sha2::{Digest, Sha256};
        let computed_hash = format!("{:x}", Sha256::digest(&self.content));
        computed_hash == self.content_hash
    }

    /// Get content as string (for text formats)
    pub fn content_as_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.content.clone())
    }
}

/// Release rule for configuration deployment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Release {
    pub labels: BTreeMap<String, String>,
    pub version_id: u64,
    pub priority: i32,
}

impl Release {
    /// Create a new release rule
    pub fn new(labels: BTreeMap<String, String>, version_id: u64, priority: i32) -> Self {
        Self {
            labels,
            version_id,
            priority,
        }
    }

    /// Create a default release (no labels, priority 0)
    pub fn default(version_id: u64) -> Self {
        Self {
            labels: BTreeMap::new(),
            version_id,
            priority: 0,
        }
    }

    /// Check if this release matches the given client labels
    pub fn matches(&self, client_labels: &BTreeMap<String, String>) -> bool {
        self.labels
            .iter()
            .all(|(key, value)| client_labels.get(key) == Some(value))
    }

    /// Check if this is a default release (no labels)
    pub fn is_default(&self) -> bool {
        self.labels.is_empty()
    }
}

/// Raft command enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftCommand {
    /// Create a new configuration with initial version
    CreateConfig {
        namespace: ConfigNamespace,
        name: String,
        content: Vec<u8>,
        format: ConfigFormat,
        schema: Option<String>,
        creator_id: u64,
        description: String,
    },
    /// Create a new version for an existing configuration
    CreateVersion {
        config_id: u64,
        content: Vec<u8>,
        format: Option<ConfigFormat>, // Allow format override
        creator_id: u64,
        description: String,
    },
    /// Update release rules for a configuration
    UpdateReleaseRules {
        config_id: u64,
        releases: Vec<Release>,
    },
    /// Delete a configuration and all its versions
    DeleteConfig { config_id: u64 },
    /// Delete specific versions (for cleanup/GC)
    DeleteVersions {
        config_id: u64,
        version_ids: Vec<u64>,
    },
}

impl RaftCommand {
    /// Get the config_id that this command operates on (if applicable)
    pub fn config_id(&self) -> Option<u64> {
        match self {
            RaftCommand::CreateConfig { .. } => None, // New config, no ID yet
            RaftCommand::CreateVersion { config_id, .. } => Some(*config_id),
            RaftCommand::UpdateReleaseRules { config_id, .. } => Some(*config_id),
            RaftCommand::DeleteConfig { config_id } => Some(*config_id),
            RaftCommand::DeleteVersions { config_id, .. } => Some(*config_id),
        }
    }

    /// Get the creator_id for this command (if applicable)
    pub fn creator_id(&self) -> Option<u64> {
        match self {
            RaftCommand::CreateConfig { creator_id, .. } => Some(*creator_id),
            RaftCommand::CreateVersion { creator_id, .. } => Some(*creator_id),
            RaftCommand::UpdateReleaseRules { .. } => None,
            RaftCommand::DeleteConfig { .. } => None,
            RaftCommand::DeleteVersions { .. } => None,
        }
    }

    /// Check if this command modifies configuration content
    pub fn modifies_content(&self) -> bool {
        matches!(
            self,
            RaftCommand::CreateConfig { .. } | RaftCommand::CreateVersion { .. }
        )
    }

    /// Check if this command modifies release rules
    pub fn modifies_releases(&self) -> bool {
        matches!(self, RaftCommand::UpdateReleaseRules { .. })
    }
}

/// Client request wrapper for Raft
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRequest {
    pub command: RaftCommand,
}

/// Client response for write operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientWriteResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// Declare Raft types using openraft macro
openraft::declare_raft_types!(
    pub TypeConfig:
        D = ClientRequest,
        R = ClientWriteResponse,
        NodeId = NodeId,
        Node = Node,
        SnapshotData = std::io::Cursor<Vec<u8>>,
);

/// Type alias for the Raft instance
pub type ConfluxRaft = Raft<TypeConfig>;

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
