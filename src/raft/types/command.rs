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