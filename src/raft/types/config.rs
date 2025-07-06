use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use super::helpers::make_config_key;

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
