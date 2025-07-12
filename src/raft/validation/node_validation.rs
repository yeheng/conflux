//! 节点验证模块
//!
//! 提供节点ID和地址的验证功能

use super::config::ValidationConfig;
use crate::error::{ConfluxError, Result};
use crate::raft::types::NodeId;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use tracing::debug;

/// 节点验证器
///
/// 专门负责节点ID和地址的验证
pub struct NodeValidator<'a> {
    config: &'a ValidationConfig,
}

impl<'a> NodeValidator<'a> {
    /// 创建新的节点验证器
    ///
    /// # Arguments
    ///
    /// * `config` - 验证配置
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, NodeValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = NodeValidator::new(&config);
    /// ```
    pub fn new(config: &'a ValidationConfig) -> Self {
        Self { config }
    }

    /// 验证节点ID
    ///
    /// 检查节点ID是否在允许的范围内
    ///
    /// # Arguments
    ///
    /// * `node_id` - 要验证的节点ID
    ///
    /// # Returns
    ///
    /// 如果验证通过返回Ok(())，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, NodeValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = NodeValidator::new(&config);
    ///
    /// assert!(validator.validate_node_id(1).is_ok());
    /// assert!(validator.validate_node_id(0).is_err());
    /// ```
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

    /// 验证节点地址字符串
    ///
    /// 检查地址格式、端口范围和IP地址有效性
    ///
    /// # Arguments
    ///
    /// * `address` - 要验证的地址字符串
    ///
    /// # Returns
    ///
    /// 如果验证通过返回解析后的SocketAddr，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, NodeValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = NodeValidator::new(&config);
    ///
    /// let addr = validator.validate_node_address("127.0.0.1:8080").unwrap();
    /// assert_eq!(addr.port(), 8080);
    /// ```
    pub fn validate_node_address(&self, address: &str) -> Result<SocketAddr> {
        debug!("Validating node address: {}", address);

        if address.is_empty() {
            return Err(ConfluxError::validation(
                "Node address cannot be empty".to_string(),
            ));
        }

        if address.len() > self.config.max_hostname_length + 10 {
            // +10 for port
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

    /// 验证IP地址
    ///
    /// 检查IP地址是否符合配置的网络策略
    ///
    /// # Arguments
    ///
    /// * `ip` - 要验证的IP地址
    ///
    /// # Returns
    ///
    /// 如果验证通过返回Ok(())，否则返回错误
    pub fn validate_ip_address(&self, ip: IpAddr) -> Result<()> {
        debug!("Validating IP address: {}", ip);

        match ip {
            IpAddr::V4(ipv4) => {
                if ipv4.is_loopback() && !self.config.allow_localhost {
                    return Err(ConfluxError::validation(
                        "Localhost addresses are not allowed".to_string(),
                    ));
                }

                if ipv4.is_private() && !self.config.allow_private_ips {
                    return Err(ConfluxError::validation(
                        "Private IP addresses are not allowed".to_string(),
                    ));
                }

                if ipv4.is_unspecified() {
                    return Err(ConfluxError::validation(
                        "Unspecified IP address (0.0.0.0) is not allowed".to_string(),
                    ));
                }

                if ipv4.is_broadcast() {
                    return Err(ConfluxError::validation(
                        "Broadcast IP address is not allowed".to_string(),
                    ));
                }

                if ipv4.is_multicast() {
                    return Err(ConfluxError::validation(
                        "Multicast IP address is not allowed".to_string(),
                    ));
                }
            }
            IpAddr::V6(ipv6) => {
                if ipv6.is_loopback() && !self.config.allow_localhost {
                    return Err(ConfluxError::validation(
                        "Localhost addresses are not allowed".to_string(),
                    ));
                }

                if ipv6.is_unspecified() {
                    return Err(ConfluxError::validation(
                        "Unspecified IP address (::) is not allowed".to_string(),
                    ));
                }

                if ipv6.is_multicast() {
                    return Err(ConfluxError::validation(
                        "Multicast IP address is not allowed".to_string(),
                    ));
                }

                // Check for private/local addresses in IPv6
                if !self.config.allow_private_ips {
                    let octets = ipv6.octets();
                    // Check for unique local addresses (fc00::/7)
                    if octets[0] == 0xfc || octets[0] == 0xfd {
                        return Err(ConfluxError::validation(
                            "Private IPv6 addresses are not allowed".to_string(),
                        ));
                    }
                    // Check for link-local addresses (fe80::/10)
                    if octets[0] == 0xfe && (octets[1] & 0xc0) == 0x80 {
                        return Err(ConfluxError::validation(
                            "Link-local IPv6 addresses are not allowed".to_string(),
                        ));
                    }
                }
            }
        }

        debug!("IP address {} is valid", ip);
        Ok(())
    }

    /// 验证节点ID的唯一性
    ///
    /// 检查节点ID是否在现有集群中已存在
    ///
    /// # Arguments
    ///
    /// * `node_id` - 要验证的节点ID
    /// * `existing_nodes` - 现有节点ID列表
    ///
    /// # Returns
    ///
    /// 如果节点ID唯一返回Ok(())，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, NodeValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = NodeValidator::new(&config);
    /// let existing_nodes = vec![1, 2, 3];
    ///
    /// assert!(validator.validate_node_id_uniqueness(4, &existing_nodes).is_ok());
    /// assert!(validator.validate_node_id_uniqueness(2, &existing_nodes).is_err());
    /// ```
    pub fn validate_node_id_uniqueness(
        &self,
        node_id: NodeId,
        existing_nodes: &[NodeId],
    ) -> Result<()> {
        debug!(
            "Validating node ID {} uniqueness against {} existing nodes",
            node_id,
            existing_nodes.len()
        );

        if existing_nodes.contains(&node_id) {
            return Err(ConfluxError::validation(format!(
                "Node ID {} already exists in cluster",
                node_id
            )));
        }

        debug!("Node ID {} is unique", node_id);
        Ok(())
    }

    /// 验证地址的唯一性
    ///
    /// 检查地址是否在现有集群中已存在
    ///
    /// # Arguments
    ///
    /// * `address` - 要验证的地址
    /// * `existing_addresses` - 现有地址列表
    ///
    /// # Returns
    ///
    /// 如果地址唯一返回Ok(())，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, NodeValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = NodeValidator::new(&config);
    /// let existing_addresses = vec!["127.0.0.1:8080".to_string(), "127.0.0.1:8081".to_string()];
    ///
    /// assert!(validator.validate_address_uniqueness("127.0.0.1:8082", &existing_addresses).is_ok());
    /// assert!(validator.validate_address_uniqueness("127.0.0.1:8080", &existing_addresses).is_err());
    /// ```
    pub fn validate_address_uniqueness(
        &self,
        address: &str,
        existing_addresses: &[String],
    ) -> Result<()> {
        debug!(
            "Validating address {} uniqueness against {} existing addresses",
            address,
            existing_addresses.len()
        );

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_node_id() {
        let config = ValidationConfig::default();
        let validator = NodeValidator::new(&config);

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
        let config = ValidationConfig::default();
        let validator = NodeValidator::new(&config);

        // Valid addresses
        assert!(validator.validate_node_address("127.0.0.1:8080").is_ok());
        assert!(validator
            .validate_node_address("192.168.1.100:3000")
            .is_ok());
        assert!(validator.validate_node_address("[::1]:8080").is_ok());

        // Invalid addresses
        assert!(validator.validate_node_address("").is_err());
        assert!(validator.validate_node_address("invalid").is_err());
        assert!(validator.validate_node_address("127.0.0.1:99999").is_err());
        assert!(validator.validate_node_address("127.0.0.1:80").is_err()); // Port too low
    }

    #[test]
    fn test_validate_node_id_uniqueness() {
        let config = ValidationConfig::default();
        let validator = NodeValidator::new(&config);
        let existing_nodes = vec![1, 2, 3];

        // Unique node ID
        assert!(validator
            .validate_node_id_uniqueness(4, &existing_nodes)
            .is_ok());

        // Duplicate node ID
        assert!(validator
            .validate_node_id_uniqueness(2, &existing_nodes)
            .is_err());
    }

    #[test]
    fn test_validate_address_uniqueness() {
        let config = ValidationConfig::default();
        let validator = NodeValidator::new(&config);
        let existing_addresses = vec!["127.0.0.1:8080".to_string(), "127.0.0.1:8081".to_string()];

        // Unique address
        assert!(validator
            .validate_address_uniqueness("127.0.0.1:8082", &existing_addresses)
            .is_ok());

        // Duplicate address
        assert!(validator
            .validate_address_uniqueness("127.0.0.1:8080", &existing_addresses)
            .is_err());
    }

    #[test]
    fn test_strict_network_policy() {
        let mut config = ValidationConfig::default();
        config.allow_localhost = false;
        config.allow_private_ips = false;

        let validator = NodeValidator::new(&config);

        // These should fail with strict policy
        assert!(validator.validate_node_address("127.0.0.1:8080").is_err());
        assert!(validator.validate_node_address("192.168.1.1:8080").is_err());
        assert!(validator.validate_node_address("10.0.0.1:8080").is_err());
    }
}
