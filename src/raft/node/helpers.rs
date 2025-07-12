//! Raft节点辅助函数模块
//!
//! 提供创建和配置Raft节点的便利函数

use super::config::{NodeConfig, ResourceLimits};
use crate::raft::{network::NetworkConfig, types::NodeId};
use openraft::Config as RaftConfig;

/// 创建基本的节点配置
/// 
/// 使用默认的Raft配置、网络配置和资源限制创建节点配置
/// 
/// # Arguments
/// 
/// * `node_id` - 节点ID，在集群中必须唯一
/// * `address` - 节点网络地址
/// 
/// # Returns
/// 
/// 返回配置好的NodeConfig实例
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::node::create_node_config;
/// 
/// let config = create_node_config(1, "127.0.0.1:8080".to_string());
/// assert_eq!(config.node_id, 1);
/// assert_eq!(config.address, "127.0.0.1:8080");
/// ```
pub fn create_node_config(node_id: NodeId, address: String) -> NodeConfig {
    NodeConfig {
        node_id,
        address,
        raft_config: RaftConfig::default(),
        network_config: NetworkConfig::default(),
        heartbeat_interval: 150,
        election_timeout_min: 300,
        election_timeout_max: 600,
        resource_limits: ResourceLimits::default(),
    }
}

/// 创建带有自定义超时配置的节点配置
/// 
/// # Arguments
/// 
/// * `node_id` - 节点ID
/// * `address` - 节点网络地址
/// * `heartbeat_interval` - 心跳间隔（毫秒）
/// * `election_timeout_min` - 选举超时最小值（毫秒）
/// * `election_timeout_max` - 选举超时最大值（毫秒）
/// 
/// # Returns
/// 
/// 返回配置好的NodeConfig实例
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::node::create_node_config_with_timeouts;
/// 
/// let config = create_node_config_with_timeouts(
///     1, 
///     "127.0.0.1:8080".to_string(),
///     100, // 心跳间隔
///     200, // 选举超时最小值
///     400  // 选举超时最大值
/// );
/// 
/// assert_eq!(config.heartbeat_interval, 100);
/// assert_eq!(config.election_timeout_min, 200);
/// assert_eq!(config.election_timeout_max, 400);
/// ```
pub fn create_node_config_with_timeouts(
    node_id: NodeId, 
    address: String,
    heartbeat_interval: u64,
    election_timeout_min: u64,
    election_timeout_max: u64,
) -> NodeConfig {
    NodeConfig {
        node_id,
        address,
        raft_config: RaftConfig::default(),
        network_config: NetworkConfig::default(),
        heartbeat_interval,
        election_timeout_min,
        election_timeout_max,
        resource_limits: ResourceLimits::default(),
    }
}

/// 创建带有自定义资源限制的节点配置
/// 
/// # Arguments
/// 
/// * `node_id` - 节点ID
/// * `address` - 节点网络地址
/// * `resource_limits` - 自定义资源限制配置
/// 
/// # Returns
/// 
/// 返回配置好的NodeConfig实例
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::node::{create_node_config_with_limits, ResourceLimits};
/// 
/// let limits = ResourceLimits::new(200, 100, 2_000_000, 100_000_000, 10000);
/// let config = create_node_config_with_limits(
///     1,
///     "127.0.0.1:8080".to_string(),
///     limits
/// );
/// 
/// assert_eq!(config.resource_limits.max_requests_per_second, 200);
/// ```
pub fn create_node_config_with_limits(
    node_id: NodeId,
    address: String,
    resource_limits: ResourceLimits,
) -> NodeConfig {
    NodeConfig {
        node_id,
        address,
        raft_config: RaftConfig::default(),
        network_config: NetworkConfig::default(),
        heartbeat_interval: 150,
        election_timeout_min: 300,
        election_timeout_max: 600,
        resource_limits,
    }
}

/// 创建完全自定义的节点配置
/// 
/// # Arguments
/// 
/// * `node_id` - 节点ID
/// * `address` - 节点网络地址
/// * `raft_config` - Raft算法配置
/// * `network_config` - 网络配置
/// * `heartbeat_interval` - 心跳间隔（毫秒）
/// * `election_timeout_min` - 选举超时最小值（毫秒）
/// * `election_timeout_max` - 选举超时最大值（毫秒）
/// * `resource_limits` - 资源限制配置
/// 
/// # Returns
/// 
/// 返回配置好的NodeConfig实例
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::node::{create_custom_node_config, ResourceLimits};
/// use conflux::raft::network::NetworkConfig;
/// use openraft::Config as RaftConfig;
/// 
/// let config = create_custom_node_config(
///     1,
///     "127.0.0.1:8080".to_string(),
///     RaftConfig::default(),
///     NetworkConfig::default(),
///     100,
///     200,
///     400,
///     ResourceLimits::default()
/// );
/// ```
pub fn create_custom_node_config(
    node_id: NodeId,
    address: String,
    raft_config: RaftConfig,
    network_config: NetworkConfig,
    heartbeat_interval: u64,
    election_timeout_min: u64,
    election_timeout_max: u64,
    resource_limits: ResourceLimits,
) -> NodeConfig {
    NodeConfig {
        node_id,
        address,
        raft_config,
        network_config,
        heartbeat_interval,
        election_timeout_min,
        election_timeout_max,
        resource_limits,
    }
}

/// 创建开发环境的节点配置
/// 
/// 使用适合开发和测试的配置参数
/// 
/// # Arguments
/// 
/// * `node_id` - 节点ID
/// * `address` - 节点网络地址
/// 
/// # Returns
/// 
/// 返回适合开发环境的NodeConfig实例
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::node::create_dev_node_config;
/// 
/// let config = create_dev_node_config(1, "127.0.0.1:8080".to_string());
/// 
/// // 开发环境使用更短的超时时间
/// assert_eq!(config.heartbeat_interval, 50);
/// assert_eq!(config.election_timeout_min, 100);
/// assert_eq!(config.election_timeout_max, 200);
/// ```
pub fn create_dev_node_config(node_id: NodeId, address: String) -> NodeConfig {
    let mut resource_limits = ResourceLimits::default();
    // 开发环境允许更多的请求
    resource_limits.max_requests_per_second = 1000;
    resource_limits.max_concurrent_requests = 200;
    
    NodeConfig {
        node_id,
        address,
        raft_config: RaftConfig::default(),
        network_config: NetworkConfig::default(),
        heartbeat_interval: 50,  // 更短的心跳间隔
        election_timeout_min: 100,  // 更短的选举超时
        election_timeout_max: 200,
        resource_limits,
    }
}

/// 创建生产环境的节点配置
/// 
/// 使用适合生产环境的保守配置参数
/// 
/// # Arguments
/// 
/// * `node_id` - 节点ID
/// * `address` - 节点网络地址
/// 
/// # Returns
/// 
/// 返回适合生产环境的NodeConfig实例
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::node::create_prod_node_config;
/// 
/// let config = create_prod_node_config(1, "127.0.0.1:8080".to_string());
/// 
/// // 生产环境使用更保守的超时时间
/// assert_eq!(config.heartbeat_interval, 200);
/// assert_eq!(config.election_timeout_min, 500);
/// assert_eq!(config.election_timeout_max, 1000);
/// ```
pub fn create_prod_node_config(node_id: NodeId, address: String) -> NodeConfig {
    let mut resource_limits = ResourceLimits::default();
    // 生产环境使用更保守的资源限制
    resource_limits.max_requests_per_second = 50;
    resource_limits.max_concurrent_requests = 25;
    resource_limits.max_request_size = 512 * 1024; // 512KB
    resource_limits.request_timeout_ms = 10000; // 10秒
    
    NodeConfig {
        node_id,
        address,
        raft_config: RaftConfig::default(),
        network_config: NetworkConfig::default(),
        heartbeat_interval: 200,  // 更长的心跳间隔
        election_timeout_min: 500,  // 更长的选举超时
        election_timeout_max: 1000,
        resource_limits,
    }
}

/// 验证节点配置的网络连通性
/// 
/// # Arguments
/// 
/// * `config` - 要验证的节点配置
/// 
/// # Returns
/// 
/// 如果配置有效返回Ok(())，否则返回错误信息
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::node::{create_node_config, validate_node_connectivity};
/// 
/// let config = create_node_config(1, "127.0.0.1:8080".to_string());
/// let result = validate_node_connectivity(&config);
/// // 注意：这个函数只做基本的格式验证，不做实际的网络连接测试
/// ```
pub fn validate_node_connectivity(config: &NodeConfig) -> Result<(), String> {
    // 验证地址格式
    if !config.address.contains(':') {
        return Err("Address must contain port (format: host:port)".to_string());
    }
    
    let parts: Vec<&str> = config.address.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid address format (expected host:port)".to_string());
    }
    
    // 验证端口号
    if let Err(_) = parts[1].parse::<u16>() {
        return Err("Invalid port number".to_string());
    }
    
    // 验证主机名/IP不为空
    if parts[0].is_empty() {
        return Err("Host cannot be empty".to_string());
    }
    
    Ok(())
}

/// 比较两个节点配置是否兼容
/// 
/// 检查两个节点配置是否可以在同一个集群中工作
/// 
/// # Arguments
/// 
/// * `config1` - 第一个节点配置
/// * `config2` - 第二个节点配置
/// 
/// # Returns
/// 
/// 如果配置兼容返回Ok(())，否则返回错误信息
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::node::{create_node_config, compare_node_configs};
/// 
/// let config1 = create_node_config(1, "127.0.0.1:8080".to_string());
/// let config2 = create_node_config(2, "127.0.0.1:8081".to_string());
/// 
/// let result = compare_node_configs(&config1, &config2);
/// assert!(result.is_ok());
/// ```
pub fn compare_node_configs(config1: &NodeConfig, config2: &NodeConfig) -> Result<(), String> {
    // 节点ID必须不同
    if config1.node_id == config2.node_id {
        return Err("Node IDs must be different".to_string());
    }
    
    // 地址必须不同
    if config1.address == config2.address {
        return Err("Node addresses must be different".to_string());
    }
    
    // 超时配置应该相似（允许一定的差异）
    let heartbeat_diff = (config1.heartbeat_interval as i64 - config2.heartbeat_interval as i64).abs();
    if heartbeat_diff > 100 {
        return Err("Heartbeat intervals are too different (may cause cluster instability)".to_string());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_node_config() {
        let config = create_node_config(1, "127.0.0.1:8080".to_string());
        assert_eq!(config.node_id, 1);
        assert_eq!(config.address, "127.0.0.1:8080");
        assert_eq!(config.heartbeat_interval, 150);
    }

    #[test]
    fn test_create_node_config_with_timeouts() {
        let config = create_node_config_with_timeouts(1, "127.0.0.1:8080".to_string(), 100, 200, 400);
        assert_eq!(config.heartbeat_interval, 100);
        assert_eq!(config.election_timeout_min, 200);
        assert_eq!(config.election_timeout_max, 400);
    }

    #[test]
    fn test_dev_vs_prod_config() {
        let dev_config = create_dev_node_config(1, "127.0.0.1:8080".to_string());
        let prod_config = create_prod_node_config(1, "127.0.0.1:8080".to_string());
        
        // 开发环境应该有更短的超时时间
        assert!(dev_config.heartbeat_interval < prod_config.heartbeat_interval);
        assert!(dev_config.election_timeout_min < prod_config.election_timeout_min);
        
        // 开发环境应该允许更多的请求
        assert!(dev_config.resource_limits.max_requests_per_second > prod_config.resource_limits.max_requests_per_second);
    }

    #[test]
    fn test_validate_node_connectivity() {
        let valid_config = create_node_config(1, "127.0.0.1:8080".to_string());
        assert!(validate_node_connectivity(&valid_config).is_ok());
        
        let invalid_config = create_node_config(1, "invalid_address".to_string());
        assert!(validate_node_connectivity(&invalid_config).is_err());
    }

    #[test]
    fn test_compare_node_configs() {
        let config1 = create_node_config(1, "127.0.0.1:8080".to_string());
        let config2 = create_node_config(2, "127.0.0.1:8081".to_string());
        
        assert!(compare_node_configs(&config1, &config2).is_ok());
        
        // 相同的节点ID应该失败
        let config3 = create_node_config(1, "127.0.0.1:8082".to_string());
        assert!(compare_node_configs(&config1, &config3).is_err());
    }
}
