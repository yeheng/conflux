use anyhow::Result;
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub raft: RaftConfig,
    pub storage: StorageConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub observability: ObservabilityConfig,
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub request_timeout_secs: u64,
}

/// Raft consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    pub node_id: u64,
    pub cluster_name: String,
    pub data_dir: String,
    pub heartbeat_interval_ms: u64,
    pub election_timeout_ms: u64,
    pub snapshot_threshold: u64,
    pub max_applied_log_to_keep: u64,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: String,
    pub max_open_files: i32,
    pub cache_size_mb: usize,
    pub write_buffer_size_mb: usize,
    pub max_write_buffer_number: i32,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,
    pub enable_mtls: bool,
    pub cert_file: Option<String>,
    pub key_file: Option<String>,
    pub ca_file: Option<String>,
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub metrics_enabled: bool,
    pub metrics_port: u16,
    pub tracing_enabled: bool,
    pub tracing_endpoint: Option<String>,
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                max_connections: 1000,
                request_timeout_secs: 30,
            },
            raft: RaftConfig {
                node_id: 1,
                cluster_name: "conflux-cluster".to_string(),
                data_dir: "./data/raft".to_string(),
                heartbeat_interval_ms: 500,
                election_timeout_ms: 1500,
                snapshot_threshold: 1000,
                max_applied_log_to_keep: 1000,
            },
            storage: StorageConfig {
                data_dir: "./data/storage".to_string(),
                max_open_files: 1000,
                cache_size_mb: 256,
                write_buffer_size_mb: 64,
                max_write_buffer_number: 3,
            },
            database: DatabaseConfig {
                url: "postgres://postgres:postgres@localhost:5432/conflux".to_string(),
                max_connections: 10,
                min_connections: 1,
                connect_timeout_secs: 30,
                idle_timeout_secs: 600,
                max_lifetime_secs: 3600,
            },
            security: SecurityConfig {
                jwt_secret: "your-secret-key-change-in-production".to_string(),
                jwt_expiration_hours: 24,
                enable_mtls: false,
                cert_file: None,
                key_file: None,
                ca_file: None,
            },
            observability: ObservabilityConfig {
                metrics_enabled: true,
                metrics_port: 9090,
                tracing_enabled: true,
                tracing_endpoint: None,
                log_level: "info".to_string(),
            },
        }
    }
}

impl AppConfig {
    /// Load configuration from files and environment variables
    pub async fn load() -> Result<Self> {
        let mut config_builder = Config::builder()
            // Start with default values
            .add_source(Config::try_from(&AppConfig::default())?);

        // Add configuration files if they exist
        let config_files = [
            "config/default.toml",
            "config/local.toml",
            "/etc/conflux/config.toml",
        ];

        for config_file in &config_files {
            if Path::new(config_file).exists() {
                config_builder = config_builder.add_source(File::with_name(config_file));
            }
        }

        // Add environment variables with CONFLUX_ prefix
        config_builder = config_builder.add_source(
            Environment::with_prefix("CONFLUX")
                .separator("_")
                .try_parsing(true),
        );

        let config = config_builder.build()?;
        let app_config: AppConfig = config.try_deserialize()?;

        // Validate configuration
        app_config.validate()?;

        Ok(app_config)
    }

    /// Validate the configuration
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate server configuration
        if self.server.port == 0 {
            return Err(ConfigError::Message("Server port cannot be 0".to_string()));
        }

        // Validate Raft configuration
        if self.raft.node_id == 0 {
            return Err(ConfigError::Message("Raft node_id cannot be 0".to_string()));
        }

        if self.raft.heartbeat_interval_ms == 0 {
            return Err(ConfigError::Message(
                "Raft heartbeat_interval_ms cannot be 0".to_string(),
            ));
        }

        if self.raft.election_timeout_ms <= self.raft.heartbeat_interval_ms {
            return Err(ConfigError::Message(
                "Raft election_timeout_ms must be greater than heartbeat_interval_ms".to_string(),
            ));
        }

        // Validate database configuration
        if self.database.url.is_empty() {
            return Err(ConfigError::Message(
                "Database URL cannot be empty".to_string(),
            ));
        }

        // Validate security configuration
        if self.security.jwt_secret.is_empty() {
            return Err(ConfigError::Message(
                "JWT secret cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}
