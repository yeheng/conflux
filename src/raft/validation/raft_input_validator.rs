use crate::error::Result;
use crate::raft::types::NodeId;
use crate::raft::validation::ClusterSuggestions;
use crate::raft::{validation::ComprehensiveValidator, ValidationConfig};

/// Raft输入验证器
///
/// 提供Raft集群操作的综合输入验证功能
///
/// # Examples
///
/// ```rust
/// use conflux::raft::validation::{RaftInputValidator, ValidationConfig};
///
/// let validator = RaftInputValidator::new();
/// let result = validator.validate_add_node(1, "127.0.0.1:8080", &[]);
/// ```
pub struct RaftInputValidator {
    comprehensive_validator: ComprehensiveValidator,
}

impl RaftInputValidator {
    /// 创建新的Raft输入验证器
    ///
    /// 使用默认的验证配置
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::RaftInputValidator;
    ///
    /// let validator = RaftInputValidator::new();
    /// ```
    pub fn new() -> Self {
        Self {
            comprehensive_validator: ComprehensiveValidator::new(ValidationConfig::default()),
        }
    }

    /// 使用自定义配置创建验证器
    ///
    /// # Arguments
    ///
    /// * `config` - 验证配置
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{RaftInputValidator, ValidationConfig};
    ///
    /// let config = ValidationConfig::dev();
    /// let validator = RaftInputValidator::with_config(config);
    /// ```
    pub fn with_config(config: ValidationConfig) -> Self {
        Self {
            comprehensive_validator: ComprehensiveValidator::new(config),
        }
    }

    /// 验证添加节点操作
    ///
    /// # Arguments
    ///
    /// * `node_id` - 要添加的节点ID
    /// * `address` - 节点地址
    /// * `existing_nodes` - 现有节点列表 (节点ID, 地址)
    ///
    /// # Returns
    ///
    /// 如果验证通过返回解析后的地址，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::RaftInputValidator;
    ///
    /// let validator = RaftInputValidator::new();
    /// let existing_nodes = vec![(1, "127.0.0.1:8080".to_string())];
    ///
    /// let result = validator.validate_add_node(2, "127.0.0.1:8081", &existing_nodes);
    /// assert!(result.is_ok());
    /// ```
    pub fn validate_add_node(
        &self,
        node_id: NodeId,
        address: &str,
        existing_nodes: &[(NodeId, String)],
    ) -> Result<std::net::SocketAddr> {
        self.comprehensive_validator
            .validate_add_node(node_id, address, existing_nodes)
    }

    /// 验证移除节点操作
    ///
    /// # Arguments
    ///
    /// * `node_id` - 要移除的节点ID
    /// * `existing_nodes` - 现有节点列表
    ///
    /// # Returns
    ///
    /// 如果验证通过返回Ok(())，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::RaftInputValidator;
    ///
    /// let validator = RaftInputValidator::new();
    /// let existing_nodes = vec![(1, "127.0.0.1:8080".to_string()), (2, "127.0.0.1:8081".to_string())];
    ///
    /// let result = validator.validate_remove_node(2, &existing_nodes);
    /// assert!(result.is_ok());
    /// ```
    pub fn validate_remove_node(
        &self,
        node_id: NodeId,
        existing_nodes: &[(NodeId, String)],
    ) -> Result<()> {
        self.comprehensive_validator
            .validate_remove_node(node_id, existing_nodes)
    }

    /// 验证超时配置
    ///
    /// # Arguments
    ///
    /// * `heartbeat_interval` - 可选的心跳间隔
    /// * `election_timeout_min` - 可选的选举超时最小值
    /// * `election_timeout_max` - 可选的选举超时最大值
    ///
    /// # Returns
    ///
    /// 如果验证通过返回Ok(())，否则返误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::RaftInputValidator;
    ///
    /// let validator = RaftInputValidator::new();
    ///
    /// let result = validator.validate_timeout_config(Some(100), Some(300), Some(600));
    /// assert!(result.is_ok());
    /// ```
    pub fn validate_timeout_config(
        &self,
        heartbeat_interval: Option<u64>,
        election_timeout_min: Option<u64>,
        election_timeout_max: Option<u64>,
    ) -> Result<()> {
        self.comprehensive_validator.validate_timeout_config(
            heartbeat_interval,
            election_timeout_min,
            election_timeout_max,
        )
    }

    /// 验证集群健康状态
    ///
    /// # Arguments
    ///
    /// * `total_nodes` - 总节点数
    /// * `healthy_nodes` - 健康节点数
    ///
    /// # Returns
    ///
    /// 如果集群健康返回Ok(())，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::RaftInputValidator;
    ///
    /// let validator = RaftInputValidator::new();
    ///
    /// let result = validator.validate_cluster_health(5, 3);
    /// assert!(result.is_ok());
    /// ```
    pub fn validate_cluster_health(&self, total_nodes: usize, healthy_nodes: usize) -> Result<()> {
        self.comprehensive_validator
            .validate_cluster_health(total_nodes, healthy_nodes)
    }

    /// 获取集群建议
    ///
    /// # Arguments
    ///
    /// * `current_cluster_size` - 当前集群大小
    /// * `current_heartbeat` - 当前心跳间隔
    /// * `current_election_min` - 当前选举超时最小值
    /// * `network_latency_ms` - 网络延迟
    ///
    /// # Returns
    ///
    /// 返回集群优化建议
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::RaftInputValidator;
    ///
    /// let validator = RaftInputValidator::new();
    ///
    /// let suggestions = validator.get_cluster_suggestions(4, 100, 300, 10);
    /// println!("Suggestions: {:?}", suggestions);
    /// ```
    pub fn get_cluster_suggestions(
        &self,
        current_cluster_size: usize,
        current_heartbeat: u64,
        current_election_min: u64,
        network_latency_ms: u64,
    ) -> ClusterSuggestions {
        self.comprehensive_validator.get_cluster_suggestions(
            current_cluster_size,
            current_heartbeat,
            current_election_min,
            network_latency_ms,
        )
    }

    /// 获取验证配置
    ///
    /// # Returns
    ///
    /// 返回当前的验证配置
    pub fn get_config(&self) -> &ValidationConfig {
        self.comprehensive_validator.get_config()
    }
}

impl Default for RaftInputValidator {
    fn default() -> Self {
        Self::new()
    }
}