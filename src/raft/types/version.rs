use serde::{Deserialize, Serialize};
use super::config::ConfigFormat;

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
