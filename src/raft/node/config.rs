//! Raft节点配置模块
//!
//! 定义节点配置和资源限制相关的数据结构

use crate::raft::{network::NetworkConfig, types::NodeId};
use openraft::Config as RaftConfig;

/// Raft节点配置
/// 
/// 包含节点运行所需的所有配置参数，包括网络配置、超时设置和资源限制
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::node::{NodeConfig, ResourceLimits};
/// 
/// let config = NodeConfig {
///     node_id: 1,
///     address: "127.0.0.1:8080".to_string(),
///     heartbeat_interval: 150,
///     election_timeout_min: 300,
///     election_timeout_max: 600,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// 节点ID，在集群中必须唯一
    pub node_id: NodeId,
    /// 节点网络通信地址
    pub address: String,
    /// Raft算法配置
    pub raft_config: RaftConfig,
    /// 网络配置
    pub network_config: NetworkConfig,
    /// 心跳间隔（毫秒），默认150ms
    pub heartbeat_interval: u64,
    /// 选举超时最小值（毫秒），默认300ms
    pub election_timeout_min: u64,
    /// 选举超时最大值（毫秒），默认600ms
    pub election_timeout_max: u64,
    /// 资源限制配置
    pub resource_limits: ResourceLimits,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: 1,
            address: "127.0.0.1:8080".to_string(),
            raft_config: RaftConfig::default(),
            network_config: NetworkConfig::default(),
            heartbeat_interval: 150,
            election_timeout_min: 300,
            election_timeout_max: 600,
            resource_limits: ResourceLimits::default(),
        }
    }
}

/// 客户端请求资源限制配置
/// 
/// 用于控制客户端请求的频率、大小和并发数，防止资源滥用
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::node::ResourceLimits;
/// 
/// let limits = ResourceLimits {
///     max_requests_per_second: 200,
///     max_concurrent_requests: 100,
///     max_request_size: 2 * 1024 * 1024, // 2MB
///     max_memory_usage: 100 * 1024 * 1024, // 100MB
///     request_timeout_ms: 10000, // 10 seconds
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// 每个客户端每秒最大请求数
    pub max_requests_per_second: u32,
    /// 最大并发请求数
    pub max_concurrent_requests: u32,
    /// 单个请求最大大小（字节）
    pub max_request_size: usize,
    /// 待处理请求最大内存使用量（字节）
    pub max_memory_usage: usize,
    /// 请求超时时间（毫秒）
    pub request_timeout_ms: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_requests_per_second: 100,
            max_concurrent_requests: 50,
            max_request_size: 1024 * 1024, // 1MB
            max_memory_usage: 50 * 1024 * 1024, // 50MB
            request_timeout_ms: 5000, // 5 seconds
        }
    }
}

impl ResourceLimits {
    /// 创建新的资源限制配置
    /// 
    /// # Arguments
    /// 
    /// * `max_requests_per_second` - 每秒最大请求数
    /// * `max_concurrent_requests` - 最大并发请求数
    /// * `max_request_size` - 单个请求最大大小
    /// * `max_memory_usage` - 最大内存使用量
    /// * `request_timeout_ms` - 请求超时时间
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::node::ResourceLimits;
    /// 
    /// let limits = ResourceLimits::new(200, 100, 2_000_000, 100_000_000, 10000);
    /// ```
    pub fn new(
        max_requests_per_second: u32,
        max_concurrent_requests: u32,
        max_request_size: usize,
        max_memory_usage: usize,
        request_timeout_ms: u64,
    ) -> Self {
        Self {
            max_requests_per_second,
            max_concurrent_requests,
            max_request_size,
            max_memory_usage,
            request_timeout_ms,
        }
    }

    /// 验证资源限制配置的合理性
    /// 
    /// # Returns
    /// 
    /// 如果配置合理返回Ok(())，否则返回错误信息
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::node::ResourceLimits;
    /// 
    /// let limits = ResourceLimits::default();
    /// assert!(limits.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.max_requests_per_second == 0 {
            return Err("max_requests_per_second must be greater than 0".to_string());
        }
        
        if self.max_concurrent_requests == 0 {
            return Err("max_concurrent_requests must be greater than 0".to_string());
        }
        
        if self.max_request_size == 0 {
            return Err("max_request_size must be greater than 0".to_string());
        }
        
        if self.max_memory_usage == 0 {
            return Err("max_memory_usage must be greater than 0".to_string());
        }
        
        if self.request_timeout_ms == 0 {
            return Err("request_timeout_ms must be greater than 0".to_string());
        }
        
        // 检查内存使用量是否合理（至少能容纳一个最大请求）
        if self.max_memory_usage < self.max_request_size {
            return Err("max_memory_usage must be at least max_request_size".to_string());
        }
        
        Ok(())
    }
}

impl NodeConfig {
    /// 创建新的节点配置
    /// 
    /// # Arguments
    /// 
    /// * `node_id` - 节点ID
    /// * `address` - 节点地址
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::node::NodeConfig;
    /// 
    /// let config = NodeConfig::new(1, "127.0.0.1:8080".to_string());
    /// ```
    pub fn new(node_id: NodeId, address: String) -> Self {
        Self {
            node_id,
            address,
            ..Default::default()
        }
    }

    /// 设置超时配置
    /// 
    /// # Arguments
    /// 
    /// * `heartbeat_interval` - 心跳间隔（毫秒）
    /// * `election_timeout_min` - 选举超时最小值（毫秒）
    /// * `election_timeout_max` - 选举超时最大值（毫秒）
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::node::NodeConfig;
    /// 
    /// let mut config = NodeConfig::default();
    /// config.set_timeouts(100, 200, 400);
    /// ```
    pub fn set_timeouts(&mut self, heartbeat_interval: u64, election_timeout_min: u64, election_timeout_max: u64) {
        self.heartbeat_interval = heartbeat_interval;
        self.election_timeout_min = election_timeout_min;
        self.election_timeout_max = election_timeout_max;
    }

    /// 设置资源限制
    /// 
    /// # Arguments
    /// 
    /// * `resource_limits` - 资源限制配置
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::node::{NodeConfig, ResourceLimits};
    /// 
    /// let mut config = NodeConfig::default();
    /// let limits = ResourceLimits::new(200, 100, 2_000_000, 100_000_000, 10000);
    /// config.set_resource_limits(limits);
    /// ```
    pub fn set_resource_limits(&mut self, resource_limits: ResourceLimits) {
        self.resource_limits = resource_limits;
    }

    /// 验证节点配置的合理性
    /// 
    /// # Returns
    /// 
    /// 如果配置合理返回Ok(())，否则返回错误信息
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::node::NodeConfig;
    /// 
    /// let config = NodeConfig::default();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.node_id == 0 {
            return Err("node_id must be greater than 0".to_string());
        }
        
        if self.address.is_empty() {
            return Err("address cannot be empty".to_string());
        }
        
        if self.heartbeat_interval == 0 {
            return Err("heartbeat_interval must be greater than 0".to_string());
        }
        
        if self.election_timeout_min == 0 {
            return Err("election_timeout_min must be greater than 0".to_string());
        }
        
        if self.election_timeout_max == 0 {
            return Err("election_timeout_max must be greater than 0".to_string());
        }
        
        if self.election_timeout_min >= self.election_timeout_max {
            return Err("election_timeout_min must be less than election_timeout_max".to_string());
        }
        
        if self.heartbeat_interval >= self.election_timeout_min {
            return Err("heartbeat_interval must be less than election_timeout_min".to_string());
        }
        
        // 验证资源限制
        self.resource_limits.validate()?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_requests_per_second, 100);
        assert_eq!(limits.max_concurrent_requests, 50);
        assert_eq!(limits.max_request_size, 1024 * 1024);
        assert_eq!(limits.max_memory_usage, 50 * 1024 * 1024);
        assert_eq!(limits.request_timeout_ms, 5000);
    }

    #[test]
    fn test_resource_limits_validation() {
        let mut limits = ResourceLimits::default();
        assert!(limits.validate().is_ok());

        limits.max_requests_per_second = 0;
        assert!(limits.validate().is_err());

        limits = ResourceLimits::default();
        limits.max_memory_usage = 100; // Less than max_request_size
        assert!(limits.validate().is_err());
    }

    #[test]
    fn test_node_config_default() {
        let config = NodeConfig::default();
        assert_eq!(config.node_id, 1);
        assert_eq!(config.address, "127.0.0.1:8080");
        assert_eq!(config.heartbeat_interval, 150);
        assert_eq!(config.election_timeout_min, 300);
        assert_eq!(config.election_timeout_max, 600);
    }

    #[test]
    fn test_node_config_validation() {
        let mut config = NodeConfig::default();
        assert!(config.validate().is_ok());

        config.node_id = 0;
        assert!(config.validate().is_err());

        config = NodeConfig::default();
        config.address = String::new();
        assert!(config.validate().is_err());

        config = NodeConfig::default();
        config.election_timeout_min = 600;
        config.election_timeout_max = 300;
        assert!(config.validate().is_err());
    }
}
