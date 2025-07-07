use crate::error::Result;
use crate::raft::types::*;
use super::types::Store;

impl Store {
    /// Execute a transactional operation with rollback support
    pub(crate) async fn execute_transaction<F, R>(&self, operation: F) -> Result<R>
    where
        F: FnOnce() -> Result<R>,
    {
        // For now, this is a simple wrapper, but can be extended
        // to support more complex transaction semantics
        operation()
    }

    /// Find config by ID with better error handling
    pub(crate) async fn find_config_by_id(&self, config_id: u64) -> Result<(String, Config)> {
        let configs = self.configurations.read().await;
        let found = configs
            .iter()
            .find(|(_, config)| config.id == config_id)
            .map(|(key, config)| (key.clone(), config.clone()));

        match found {
            Some((key, config)) => Ok((key, config)),
            None => Err(crate::error::ConfluxError::validation(format!(
                "Configuration with ID {} not found",
                config_id
            ))),
        }
    }

    /// Validate version exists for config
    pub(crate) async fn validate_version_exists(&self, config_id: u64, version_id: u64) -> Result<()> {
        let versions = self.versions.read().await;
        let version_exists = versions
            .get(&config_id)
            .map(|config_versions| config_versions.contains_key(&version_id))
            .unwrap_or(false);

        if !version_exists {
            return Err(crate::error::ConfluxError::validation(format!(
                "Version {} does not exist for config {}",
                version_id, config_id
            )));
        }
        Ok(())
    }

    /// Create a standardized error response
    pub(crate) fn create_error_response(message: String) -> ClientWriteResponse {
        ClientWriteResponse {
            success: false,
            message,
            data: None,
        }
    }

    /// Create a standardized success response
    pub(crate) fn create_success_response(message: String, data: Option<serde_json::Value>) -> ClientWriteResponse {
        ClientWriteResponse {
            success: true,
            message,
            data,
        }
    }

    /// Parse config name from config key more robustly
    pub(crate) fn parse_config_name_from_key(config_key: &str) -> String {
        config_key
            .split('/')
            .last()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}
