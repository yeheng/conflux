//! Input validation module for Raft cluster operations
//!
//! Provides comprehensive validation for node IDs, addresses, and other cluster parameters

use crate::error::{ConfluxError, Result};
use crate::raft::types::NodeId;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use tracing::debug;

/// Validation configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Minimum allowed node ID
    pub min_node_id: NodeId,
    /// Maximum allowed node ID
    pub max_node_id: NodeId,
    /// Allowed port range for node addresses
    pub allowed_port_range: (u16, u16),
    /// Maximum hostname length
    pub max_hostname_length: usize,
    /// Whether to allow localhost addresses
    pub allow_localhost: bool,
    /// Whether to allow private IP addresses
    pub allow_private_ips: bool,
    /// Maximum cluster size
    pub max_cluster_size: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            min_node_id: 1,
            max_node_id: 65535,
            allowed_port_range: (1024, 65535),
            max_hostname_length: 253, // RFC 1035 limit
            allow_localhost: true,
            allow_private_ips: true,
            max_cluster_size: 100,
        }
    }
}

/// Input validator for Raft operations
#[derive(Debug, Clone)]
pub struct RaftInputValidator {
    config: ValidationConfig,
}

impl RaftInputValidator {
    /// Create a new validator with default configuration
    pub fn new() -> Self {
        Self {
            config: ValidationConfig::default(),
        }
    }

    /// Create a new validator with custom configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        Self { config }
    }

    /// Validate a node ID
    pub fn validate_node_id(&self, node_id: NodeId) -> Result<()> {
        debug!("Validating node ID: {}", node_id);

        if node_id < self.config.min_node_id {
            return Err(ConfluxError::validation(format!(
                "Node ID {} is below minimum allowed value {}",
                node_id, self.config.min_node_id
            )));
        }

        if node_id > self.config.max_node_id {
            return Err(ConfluxError::validation(format!(
                "Node ID {} exceeds maximum allowed value {}",
                node_id, self.config.max_node_id
            )));
        }

        debug!("Node ID {} is valid", node_id);
        Ok(())
    }

    /// Validate a node address string
    pub fn validate_node_address(&self, address: &str) -> Result<SocketAddr> {
        debug!("Validating node address: {}", address);

        if address.is_empty() {
            return Err(ConfluxError::validation("Node address cannot be empty".to_string()));
        }

        if address.len() > self.config.max_hostname_length + 10 { // +10 for port
            return Err(ConfluxError::validation(format!(
                "Node address is too long: {} characters (max: {})",
                address.len(),
                self.config.max_hostname_length + 10
            )));
        }

        // Parse as socket address
        let socket_addr = SocketAddr::from_str(address).map_err(|e| {
            ConfluxError::validation(format!(
                "Invalid socket address format '{}': {}",
                address, e
            ))
        })?;

        // Validate port range
        let port = socket_addr.port();
        if port < self.config.allowed_port_range.0 || port > self.config.allowed_port_range.1 {
            return Err(ConfluxError::validation(format!(
                "Port {} is outside allowed range {}-{}",
                port, self.config.allowed_port_range.0, self.config.allowed_port_range.1
            )));
        }

        // Validate IP address
        self.validate_ip_address(socket_addr.ip())?;

        debug!("Node address {} is valid", address);
        Ok(socket_addr)
    }

    /// Validate an IP address
    fn validate_ip_address(&self, ip: IpAddr) -> Result<()> {
        debug!("Validating IP address: {}", ip);

        match ip {
            IpAddr::V4(ipv4) => {
                if ipv4.is_loopback() && !self.config.allow_localhost {
                    return Err(ConfluxError::validation(
                        "Localhost addresses are not allowed".to_string()
                    ));
                }

                if ipv4.is_private() && !self.config.allow_private_ips {
                    return Err(ConfluxError::validation(
                        "Private IP addresses are not allowed".to_string()
                    ));
                }

                if ipv4.is_unspecified() {
                    return Err(ConfluxError::validation(
                        "Unspecified IP address (0.0.0.0) is not allowed".to_string()
                    ));
                }

                if ipv4.is_broadcast() {
                    return Err(ConfluxError::validation(
                        "Broadcast IP address is not allowed".to_string()
                    ));
                }

                if ipv4.is_multicast() {
                    return Err(ConfluxError::validation(
                        "Multicast IP address is not allowed".to_string()
                    ));
                }
            }
            IpAddr::V6(ipv6) => {
                if ipv6.is_loopback() && !self.config.allow_localhost {
                    return Err(ConfluxError::validation(
                        "Localhost addresses are not allowed".to_string()
                    ));
                }

                if ipv6.is_unspecified() {
                    return Err(ConfluxError::validation(
                        "Unspecified IP address (::) is not allowed".to_string()
                    ));
                }

                if ipv6.is_multicast() {
                    return Err(ConfluxError::validation(
                        "Multicast IP address is not allowed".to_string()
                    ));
                }

                // Check for private/local addresses in IPv6
                if !self.config.allow_private_ips {
                    let octets = ipv6.octets();
                    // Check for unique local addresses (fc00::/7)
                    if octets[0] == 0xfc || octets[0] == 0xfd {
                        return Err(ConfluxError::validation(
                            "Private IPv6 addresses are not allowed".to_string()
                        ));
                    }
                    // Check for link-local addresses (fe80::/10)
                    if octets[0] == 0xfe && (octets[1] & 0xc0) == 0x80 {
                        return Err(ConfluxError::validation(
                            "Link-local IPv6 addresses are not allowed".to_string()
                        ));
                    }
                }
            }
        }

        debug!("IP address {} is valid", ip);
        Ok(())
    }

    /// Validate cluster size
    pub fn validate_cluster_size(&self, current_size: usize, adding_nodes: usize) -> Result<()> {
        let new_size = current_size + adding_nodes;
        
        debug!("Validating cluster size: current={}, adding={}, new={}", 
               current_size, adding_nodes, new_size);

        if new_size > self.config.max_cluster_size {
            return Err(ConfluxError::validation(format!(
                "Cluster size would exceed maximum: {} > {}",
                new_size, self.config.max_cluster_size
            )));
        }

        debug!("Cluster size {} is valid", new_size);
        Ok(())
    }

    /// Validate node ID uniqueness in cluster
    pub fn validate_node_id_uniqueness(&self, node_id: NodeId, existing_nodes: &[NodeId]) -> Result<()> {
        debug!("Validating node ID {} uniqueness against {} existing nodes", 
               node_id, existing_nodes.len());

        if existing_nodes.contains(&node_id) {
            return Err(ConfluxError::validation(format!(
                "Node ID {} already exists in cluster",
                node_id
            )));
        }

        debug!("Node ID {} is unique", node_id);
        Ok(())
    }

    /// Validate address uniqueness in cluster
    pub fn validate_address_uniqueness(&self, address: &str, existing_addresses: &[String]) -> Result<()> {
        debug!("Validating address {} uniqueness against {} existing addresses", 
               address, existing_addresses.len());

        // Parse the new address to normalize it
        let new_addr = self.validate_node_address(address)?;

        for existing in existing_addresses {
            if let Ok(existing_addr) = SocketAddr::from_str(existing) {
                if new_addr == existing_addr {
                    return Err(ConfluxError::validation(format!(
                        "Address {} already exists in cluster",
                        address
                    )));
                }
            }
        }

        debug!("Address {} is unique", address);
        Ok(())
    }

    /// Validate timeout configuration values
    pub fn validate_timeout_config(
        &self,
        heartbeat_interval: Option<u64>,
        election_timeout_min: Option<u64>,
        election_timeout_max: Option<u64>,
    ) -> Result<()> {
        debug!("Validating timeout configuration");

        if let Some(heartbeat) = heartbeat_interval {
            if heartbeat == 0 {
                return Err(ConfluxError::validation(
                    "Heartbeat interval cannot be zero".to_string()
                ));
            }
            if heartbeat > 10000 { // 10 seconds max
                return Err(ConfluxError::validation(
                    "Heartbeat interval cannot exceed 10000ms".to_string()
                ));
            }
        }

        if let Some(min_timeout) = election_timeout_min {
            if min_timeout == 0 {
                return Err(ConfluxError::validation(
                    "Election timeout min cannot be zero".to_string()
                ));
            }
            if min_timeout > 30000 { // 30 seconds max
                return Err(ConfluxError::validation(
                    "Election timeout min cannot exceed 30000ms".to_string()
                ));
            }
        }

        if let Some(max_timeout) = election_timeout_max {
            if max_timeout == 0 {
                return Err(ConfluxError::validation(
                    "Election timeout max cannot be zero".to_string()
                ));
            }
            if max_timeout > 60000 { // 60 seconds max
                return Err(ConfluxError::validation(
                    "Election timeout max cannot exceed 60000ms".to_string()
                ));
            }
        }

        // Validate relationships between timeouts
        match (heartbeat_interval, election_timeout_min, election_timeout_max) {
            (Some(heartbeat), Some(min_timeout), _) => {
                if heartbeat >= min_timeout {
                    return Err(ConfluxError::validation(
                        "Heartbeat interval must be less than election timeout min".to_string()
                    ));
                }
            }
            (Some(heartbeat), _, Some(max_timeout)) => {
                if heartbeat >= max_timeout {
                    return Err(ConfluxError::validation(
                        "Heartbeat interval must be less than election timeout max".to_string()
                    ));
                }
            }
            (_, Some(min_timeout), Some(max_timeout)) => {
                if min_timeout >= max_timeout {
                    return Err(ConfluxError::validation(
                        "Election timeout min must be less than max".to_string()
                    ));
                }
            }
            _ => {} // Partial validation, relationships will be checked later
        }

        debug!("Timeout configuration is valid");
        Ok(())
    }

    /// Comprehensive validation for adding a node
    pub fn validate_add_node(
        &self,
        node_id: NodeId,
        address: &str,
        existing_nodes: &[(NodeId, String)],
    ) -> Result<SocketAddr> {
        debug!("Performing comprehensive validation for adding node {} at {}", 
               node_id, address);

        // Validate node ID
        self.validate_node_id(node_id)?;

        // Validate address format
        let socket_addr = self.validate_node_address(address)?;

        // Validate cluster size
        self.validate_cluster_size(existing_nodes.len(), 1)?;

        // Extract existing node IDs and addresses
        let existing_node_ids: Vec<NodeId> = existing_nodes.iter().map(|(id, _)| *id).collect();
        let existing_addresses: Vec<String> = existing_nodes.iter().map(|(_, addr)| addr.clone()).collect();

        // Validate uniqueness
        self.validate_node_id_uniqueness(node_id, &existing_node_ids)?;
        self.validate_address_uniqueness(address, &existing_addresses)?;

        debug!("Node addition validation passed for node {} at {}", node_id, address);
        Ok(socket_addr)
    }

    /// Validate node removal
    pub fn validate_remove_node(
        &self,
        node_id: NodeId,
        existing_nodes: &[(NodeId, String)],
    ) -> Result<()> {
        debug!("Validating node removal for node {}", node_id);

        // Validate node ID format
        self.validate_node_id(node_id)?;

        // Check if node exists
        if !existing_nodes.iter().any(|(id, _)| *id == node_id) {
            return Err(ConfluxError::validation(format!(
                "Node ID {} does not exist in cluster",
                node_id
            )));
        }

        // Check if this would leave cluster empty
        if existing_nodes.len() <= 1 {
            return Err(ConfluxError::validation(
                "Cannot remove the last node from cluster".to_string()
            ));
        }

        debug!("Node removal validation passed for node {}", node_id);
        Ok(())
    }

    /// Update validation configuration
    pub fn update_config(&mut self, new_config: ValidationConfig) {
        debug!("Updating validation configuration");
        self.config = new_config;
    }

    /// Get current validation configuration
    pub fn get_config(&self) -> &ValidationConfig {
        &self.config
    }
}

impl Default for RaftInputValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper trait for validation error generation
trait ValidationError {
    fn validation(message: String) -> Self;
}

impl ValidationError for ConfluxError {
    fn validation(message: String) -> Self {
        ConfluxError::validation(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_node_id() {
        let validator = RaftInputValidator::new();
        
        // Valid node IDs
        assert!(validator.validate_node_id(1).is_ok());
        assert!(validator.validate_node_id(100).is_ok());
        assert!(validator.validate_node_id(65535).is_ok());
        
        // Invalid node IDs
        assert!(validator.validate_node_id(0).is_err());
        assert!(validator.validate_node_id(65536).is_err());
    }

    #[test]
    fn test_validate_node_address() {
        let validator = RaftInputValidator::new();
        
        // Valid addresses
        assert!(validator.validate_node_address("127.0.0.1:8080").is_ok());
        assert!(validator.validate_node_address("192.168.1.100:3000").is_ok());
        assert!(validator.validate_node_address("[::1]:8080").is_ok());
        
        // Invalid addresses
        assert!(validator.validate_node_address("").is_err());
        assert!(validator.validate_node_address("invalid").is_err());
        assert!(validator.validate_node_address("127.0.0.1:99999").is_err());
        assert!(validator.validate_node_address("127.0.0.1:80").is_err()); // Port too low
    }

    #[test]
    fn test_validate_cluster_size() {
        let validator = RaftInputValidator::new();
        
        // Valid cluster sizes
        assert!(validator.validate_cluster_size(5, 1).is_ok());
        assert!(validator.validate_cluster_size(99, 1).is_ok());
        
        // Invalid cluster sizes
        assert!(validator.validate_cluster_size(100, 1).is_err());
        assert!(validator.validate_cluster_size(150, 0).is_err());
    }

    #[test]
    fn test_validate_node_id_uniqueness() {
        let validator = RaftInputValidator::new();
        let existing_nodes = vec![1, 2, 3];
        
        // Unique node ID
        assert!(validator.validate_node_id_uniqueness(4, &existing_nodes).is_ok());
        
        // Duplicate node ID
        assert!(validator.validate_node_id_uniqueness(2, &existing_nodes).is_err());
    }

    #[test]
    fn test_validate_timeout_config() {
        let validator = RaftInputValidator::new();
        
        // Valid timeouts
        assert!(validator.validate_timeout_config(Some(100), Some(300), Some(600)).is_ok());
        assert!(validator.validate_timeout_config(Some(50), None, None).is_ok());
        
        // Invalid timeouts
        assert!(validator.validate_timeout_config(Some(0), None, None).is_err());
        assert!(validator.validate_timeout_config(Some(500), Some(300), None).is_err()); // heartbeat >= min
        assert!(validator.validate_timeout_config(None, Some(600), Some(300)).is_err()); // min >= max
    }

    #[test]
    fn test_comprehensive_add_node_validation() {
        let validator = RaftInputValidator::new();
        let existing_nodes = vec![(1, "127.0.0.1:8080".to_string()), (2, "127.0.0.1:8081".to_string())];
        
        // Valid addition
        assert!(validator.validate_add_node(3, "127.0.0.1:8082", &existing_nodes).is_ok());
        
        // Invalid additions
        assert!(validator.validate_add_node(1, "127.0.0.1:8082", &existing_nodes).is_err()); // Duplicate ID
        assert!(validator.validate_add_node(3, "127.0.0.1:8080", &existing_nodes).is_err()); // Duplicate address
        assert!(validator.validate_add_node(0, "127.0.0.1:8082", &existing_nodes).is_err()); // Invalid ID
    }

    #[test]
    fn test_validate_remove_node() {
        let validator = RaftInputValidator::new();
        let existing_nodes = vec![(1, "127.0.0.1:8080".to_string()), (2, "127.0.0.1:8081".to_string())];
        
        // Valid removal
        assert!(validator.validate_remove_node(1, &existing_nodes).is_ok());
        
        // Invalid removal
        assert!(validator.validate_remove_node(3, &existing_nodes).is_err()); // Node doesn't exist
        
        // Cannot remove last node
        let single_node = vec![(1, "127.0.0.1:8080".to_string())];
        assert!(validator.validate_remove_node(1, &single_node).is_err());
    }
}