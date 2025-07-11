use crate::raft::{ConfigFormat, Release};

use super::config::ConfigNamespace;
use serde::{Deserialize, Serialize};

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
    /// Update an existing configuration
    UpdateConfig {
        config_id: u64,
        namespace: ConfigNamespace,
        name: String,
        content: Vec<u8>,
        format: ConfigFormat,
        schema: Option<String>,
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
    /// Release a specific version
    ReleaseVersion { config_id: u64, version_id: u64 },
    /// Delete a configuration and all its versions
    DeleteConfig { config_id: u64 },
    DeleteVersions {
        config_id: u64,
        version_ids: Vec<u64>,
    },
    UpdateReleaseRules {
        config_id: u64,
        releases: Vec<Release>,
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
            RaftCommand::UpdateConfig { config_id, .. } => Some(*config_id),
            RaftCommand::ReleaseVersion { config_id, .. } => Some(*config_id),
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
            RaftCommand::UpdateConfig { .. } => None,
            RaftCommand::ReleaseVersion { .. } => None,
        }
    }

    /// Check if this command modifies configuration content
    pub fn modifies_content(&self) -> bool {
        matches!(
            self,
            RaftCommand::CreateConfig { .. }
                | RaftCommand::CreateVersion { .. }
                | RaftCommand::UpdateConfig { .. }
        )
    }

    /// Check if this command modifies release rules
    pub fn modifies_releases(&self) -> bool {
        matches!(
            self,
            RaftCommand::UpdateReleaseRules { .. } | RaftCommand::ReleaseVersion { .. }
        )
    }

    /// Estimate the memory usage of this command in bytes
    pub fn estimate_size(&self) -> usize {
        match self {
            RaftCommand::CreateConfig {
                namespace,
                name,
                content,
                format: _,
                schema,
                creator_id: _,
                description,
            } => {
                // Base size for the enum variant
                let base_size = std::mem::size_of::<RaftCommand>();
                // Namespace: 3 strings (tenant, app, env) + overhead
                let namespace_size = namespace.tenant.len() + namespace.app.len() + namespace.env.len() + 48;
                // Name string + heap allocation overhead
                let name_size = name.len() + 24;
                // Content Vec<u8> + heap allocation overhead
                let content_size = content.len() + 24;
                // Schema Option<String> + heap allocation overhead
                let schema_size = schema.as_ref().map(|s| s.len() + 24).unwrap_or(8);
                // Description string + heap allocation overhead
                let description_size = description.len() + 24;
                
                base_size + namespace_size + name_size + content_size + schema_size + description_size
            }
            RaftCommand::UpdateConfig {
                config_id: _,
                namespace,
                name,
                content,
                format: _,
                schema,
                description,
            } => {
                let base_size = std::mem::size_of::<RaftCommand>();
                let namespace_size = namespace.tenant.len() + namespace.app.len() + namespace.env.len() + 48;
                let name_size = name.len() + 24;
                let content_size = content.len() + 24;
                let schema_size = schema.as_ref().map(|s| s.len() + 24).unwrap_or(8);
                let description_size = description.len() + 24;
                
                base_size + namespace_size + name_size + content_size + schema_size + description_size
            }
            RaftCommand::CreateVersion {
                config_id: _,
                content,
                format: _,
                creator_id: _,
                description,
            } => {
                let base_size = std::mem::size_of::<RaftCommand>();
                let content_size = content.len() + 24;
                let description_size = description.len() + 24;
                
                base_size + content_size + description_size
            }
            RaftCommand::ReleaseVersion { config_id: _, version_id: _ } => {
                // Only contains two u64 values
                std::mem::size_of::<RaftCommand>()
            }
            RaftCommand::DeleteConfig { config_id: _ } => {
                // Only contains one u64 value
                std::mem::size_of::<RaftCommand>()
            }
            RaftCommand::DeleteVersions { config_id: _, version_ids } => {
                let base_size = std::mem::size_of::<RaftCommand>();
                // Vec<u64> + heap allocation overhead
                let version_ids_size = version_ids.len() * 8 + 24;
                
                base_size + version_ids_size
            }
            RaftCommand::UpdateReleaseRules { config_id: _, releases } => {
                let base_size = std::mem::size_of::<RaftCommand>();
                // Estimate size of Vec<Release>
                let releases_size = releases.iter().fold(24, |acc, release| {
                    // Each Release has: BTreeMap<String, String> + u64 + i32
                    let labels_size = release.labels.iter().fold(48, |acc, (k, v)| {
                        acc + k.len() + v.len() + 48 // key + value + BTreeMap overhead per entry
                    });
                    acc + labels_size + 16 // version_id (u64) + priority (i32) + padding
                });
                
                base_size + releases_size
            }
        }
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
    pub config_id: Option<u64>,
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl Default for ClientWriteResponse {
    fn default() -> Self {
        Self {
            config_id: None,
            success: false,
            message: "No operation performed".to_string(),
            data: None,
        }
    }
}