//! 验证配置模块
//!
//! 定义Raft集群操作的验证配置参数

use crate::raft::types::NodeId;

/// 验证配置
/// 
/// 定义了各种验证规则的参数，包括节点ID范围、端口范围、集群大小限制等
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::validation::ValidationConfig;
/// 
/// let config = ValidationConfig {
///     min_node_id: 1,
///     max_node_id: 1000,
///     allowed_port_range: (8000, 9000),
///     max_cluster_size: 50,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// 允许的最小节点ID
    pub min_node_id: NodeId,
    /// 允许的最大节点ID
    pub max_node_id: NodeId,
    /// 允许的端口范围 (最小端口, 最大端口)
    pub allowed_port_range: (u16, u16),
    /// 主机名最大长度
    pub max_hostname_length: usize,
    /// 是否允许localhost地址
    pub allow_localhost: bool,
    /// 是否允许私有IP地址
    pub allow_private_ips: bool,
    /// 集群最大大小
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

impl ValidationConfig {
    /// 创建新的验证配置
    /// 
    /// # Arguments
    /// 
    /// * `min_node_id` - 最小节点ID
    /// * `max_node_id` - 最大节点ID
    /// * `allowed_port_range` - 允许的端口范围
    /// * `max_cluster_size` - 最大集群大小
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::validation::ValidationConfig;
    /// 
    /// let config = ValidationConfig::new(1, 1000, (8000, 9000), 50);
    /// ```
    pub fn new(
        min_node_id: NodeId,
        max_node_id: NodeId,
        allowed_port_range: (u16, u16),
        max_cluster_size: usize,
    ) -> Self {
        Self {
            min_node_id,
            max_node_id,
            allowed_port_range,
            max_cluster_size,
            ..Default::default()
        }
    }

    /// 创建开发环境的验证配置
    /// 
    /// 使用更宽松的验证规则，适合开发和测试
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::validation::ValidationConfig;
    /// 
    /// let config = ValidationConfig::dev();
    /// assert!(config.allow_localhost);
    /// assert!(config.allow_private_ips);
    /// ```
    pub fn dev() -> Self {
        Self {
            min_node_id: 1,
            max_node_id: 65535,
            allowed_port_range: (1024, 65535),
            max_hostname_length: 253,
            allow_localhost: true,
            allow_private_ips: true,
            max_cluster_size: 1000, // 开发环境允许更大的集群
        }
    }

    /// 创建生产环境的验证配置
    /// 
    /// 使用更严格的验证规则，适合生产环境
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::validation::ValidationConfig;
    /// 
    /// let config = ValidationConfig::prod();
    /// assert!(!config.allow_localhost);
    /// assert!(!config.allow_private_ips);
    /// ```
    pub fn prod() -> Self {
        Self {
            min_node_id: 1,
            max_node_id: 10000,
            allowed_port_range: (8000, 9000), // 限制端口范围
            max_hostname_length: 253,
            allow_localhost: false, // 生产环境不允许localhost
            allow_private_ips: false, // 生产环境不允许私有IP
            max_cluster_size: 100,
        }
    }

    /// 验证配置的合理性
    /// 
    /// # Returns
    /// 
    /// 如果配置合理返回Ok(())，否则返回错误信息
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::validation::ValidationConfig;
    /// 
    /// let config = ValidationConfig::default();
    /// assert!(config.validate().is_ok());
    /// 
    /// let invalid_config = ValidationConfig {
    ///     min_node_id: 100,
    ///     max_node_id: 50, // max < min
    ///     ..Default::default()
    /// };
    /// assert!(invalid_config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.min_node_id == 0 {
            return Err("min_node_id cannot be zero".to_string());
        }

        if self.min_node_id >= self.max_node_id {
            return Err("min_node_id must be less than max_node_id".to_string());
        }

        if self.allowed_port_range.0 >= self.allowed_port_range.1 {
            return Err("Port range minimum must be less than maximum".to_string());
        }

        if self.allowed_port_range.0 == 0 {
            return Err("Port range minimum cannot be zero".to_string());
        }

        if self.max_hostname_length == 0 {
            return Err("max_hostname_length cannot be zero".to_string());
        }

        if self.max_hostname_length > 253 {
            return Err("max_hostname_length cannot exceed 253 (RFC 1035 limit)".to_string());
        }

        if self.max_cluster_size == 0 {
            return Err("max_cluster_size cannot be zero".to_string());
        }

        if self.max_cluster_size > 10000 {
            return Err("max_cluster_size cannot exceed 10000".to_string());
        }

        Ok(())
    }

    /// 设置节点ID范围
    /// 
    /// # Arguments
    /// 
    /// * `min` - 最小节点ID
    /// * `max` - 最大节点ID
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::validation::ValidationConfig;
    /// 
    /// let mut config = ValidationConfig::default();
    /// config.set_node_id_range(1, 1000);
    /// assert_eq!(config.min_node_id, 1);
    /// assert_eq!(config.max_node_id, 1000);
    /// ```
    pub fn set_node_id_range(&mut self, min: NodeId, max: NodeId) {
        self.min_node_id = min;
        self.max_node_id = max;
    }

    /// 设置端口范围
    /// 
    /// # Arguments
    /// 
    /// * `min` - 最小端口
    /// * `max` - 最大端口
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::validation::ValidationConfig;
    /// 
    /// let mut config = ValidationConfig::default();
    /// config.set_port_range(8000, 9000);
    /// assert_eq!(config.allowed_port_range, (8000, 9000));
    /// ```
    pub fn set_port_range(&mut self, min: u16, max: u16) {
        self.allowed_port_range = (min, max);
    }

    /// 设置网络策略
    /// 
    /// # Arguments
    /// 
    /// * `allow_localhost` - 是否允许localhost地址
    /// * `allow_private_ips` - 是否允许私有IP地址
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::validation::ValidationConfig;
    /// 
    /// let mut config = ValidationConfig::default();
    /// config.set_network_policy(false, false); // 生产环境策略
    /// assert!(!config.allow_localhost);
    /// assert!(!config.allow_private_ips);
    /// ```
    pub fn set_network_policy(&mut self, allow_localhost: bool, allow_private_ips: bool) {
        self.allow_localhost = allow_localhost;
        self.allow_private_ips = allow_private_ips;
    }

    /// 设置集群大小限制
    /// 
    /// # Arguments
    /// 
    /// * `max_size` - 最大集群大小
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::validation::ValidationConfig;
    /// 
    /// let mut config = ValidationConfig::default();
    /// config.set_max_cluster_size(50);
    /// assert_eq!(config.max_cluster_size, 50);
    /// ```
    pub fn set_max_cluster_size(&mut self, max_size: usize) {
        self.max_cluster_size = max_size;
    }
}