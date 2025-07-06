use thiserror::Error;

/// Main error type for the Conflux application
#[derive(Error, Debug)]
pub enum ConfluxError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Raft error: {0}")]
    Raft(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("RocksDB error: {0}")]
    RocksDB(#[from] rocksdb::Error),

    #[error("Bincode error: {0}")]
    Bincode(#[from] Box<bincode::ErrorKind>),
    
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Authorization error: {0}")]
    Authz(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, ConfluxError>;

impl ConfluxError {
    pub fn raft(msg: impl Into<String>) -> Self {
        Self::Raft(msg.into())
    }
    
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }
    
    pub fn auth(msg: impl Into<String>) -> Self {
        Self::Auth(msg.into())
    }
    
    pub fn authz(msg: impl Into<String>) -> Self {
        Self::Authz(msg.into())
    }
    
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
    
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}
