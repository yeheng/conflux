//! 综合验证模块
//!
//! 提供组合多个验证器的综合验证功能

use super::{
    config::ValidationConfig,
    node_validation::NodeValidator,
    cluster_validation::ClusterValidator,
    timeout_validation::TimeoutValidator,
};
use crate::error::Result;
use crate::raft::types::NodeId;
use tracing::debug;

/// 综合验证器
/// 
/// 组合所有验证器，提供完整的验证功能
pub struct ComprehensiveValidator {
    config: ValidationConfig,
    node_validator: NodeValidator<'static>,
    cluster_validator: ClusterValidator<'static>,
    timeout_validator: TimeoutValidator,
}

impl ComprehensiveValidator {
    /// 创建新的综合验证器
    /// 
    /// # Arguments
    /// 
    /// * `config` - 验证配置
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, ComprehensiveValidator};
    /// 
    /// let config = ValidationConfig::default();
    /// let validator = ComprehensiveValidator::new(config);
    /// ```
    pub fn new(config: ValidationConfig) -> Self {
        // 使用Box来避免生命周期问题
        let config_ref = Box::leak(Box::new(config.clone()));
        
        Self {
            config,
            node_validator: NodeValidator::new(config_ref),
            cluster_validator: ClusterValidator::new(config_ref),
            timeout_validator: TimeoutValidator::new(),
        }
    }

    /// 验证添加节点操作
    /// 
    /// 综合验证节点ID、地址、集群大小等
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
    /// use conflux::raft::validation::{ValidationConfig, ComprehensiveValidator};
    /// 
    /// let config = ValidationConfig::default();
    /// let validator = ComprehensiveValidator::new(config);
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
        debug!("Comprehensive validation for adding node {} at {}", node_id, address);

        // 1. 验证节点ID
        self.node_validator.validate_node_id(node_id)?;

        // 2. 验证节点ID唯一性
        let existing_node_ids: Vec<NodeId> = existing_nodes.iter().map(|(id, _)| *id).collect();
        self.node_validator.validate_node_id_uniqueness(node_id, &existing_node_ids)?;

        // 3. 验证节点地址
        let socket_addr = self.node_validator.validate_node_address(address)?;

        // 4. 验证地址唯一性
        let existing_addresses: Vec<String> = existing_nodes.iter().map(|(_, addr)| addr.clone()).collect();
        self.node_validator.validate_address_uniqueness(address, &existing_addresses)?;

        // 5. 验证集群大小
        self.cluster_validator.validate_cluster_size(existing_nodes.len(), 1)?;

        debug!("Add node validation passed for node {} at {}", node_id, address);
        Ok(socket_addr)
    }

    /// 验证移除节点操作
    /// 
    /// 综合验证节点存在性、集群最小大小等
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
    /// use conflux::raft::validation::{ValidationConfig, ComprehensiveValidator};
    /// 
    /// let config = ValidationConfig::default();
    /// let validator = ComprehensiveValidator::new(config);
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
        debug!("Comprehensive validation for removing node {}", node_id);

        // 1. 验证节点ID
        self.node_validator.validate_node_id(node_id)?;

        // 2. 验证节点存在
        self.cluster_validator.validate_node_exists(node_id, existing_nodes)?;

        // 3. 验证集群最小大小
        self.cluster_validator.validate_minimum_cluster_size(existing_nodes.len(), 1)?;

        debug!("Remove node validation passed for node {}", node_id);
        Ok(())
    }

    /// 验证超时配置
    /// 
    /// 验证心跳间隔和选举超时配置
    /// 
    /// # Arguments
    /// 
    /// * `heartbeat_interval` - 可选的心跳间隔
    /// * `election_timeout_min` - 可选的选举超时最小值
    /// * `election_timeout_max` - 可选的选举超时最大值
    /// 
    /// # Returns
    /// 
    /// 如果验证通过返回Ok(())，否则返回错误
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, ComprehensiveValidator};
    /// 
    /// let config = ValidationConfig::default();
    /// let validator = ComprehensiveValidator::new(config);
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
        debug!("Comprehensive timeout configuration validation");

        self.timeout_validator.validate_timeout_config(
            heartbeat_interval,
            election_timeout_min,
            election_timeout_max,
        )?;

        debug!("Timeout configuration validation passed");
        Ok(())
    }

    /// 验证集群健康状态
    /// 
    /// 检查集群是否有足够的健康节点
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
    /// use conflux::raft::validation::{ValidationConfig, ComprehensiveValidator};
    /// 
    /// let config = ValidationConfig::default();
    /// let validator = ComprehensiveValidator::new(config);
    /// 
    /// let result = validator.validate_cluster_health(5, 3);
    /// assert!(result.is_ok());
    /// ```
    pub fn validate_cluster_health(&self, total_nodes: usize, healthy_nodes: usize) -> Result<()> {
        debug!("Comprehensive cluster health validation");

        self.cluster_validator.validate_cluster_health(total_nodes, healthy_nodes)?;

        debug!("Cluster health validation passed");
        Ok(())
    }

    /// 获取集群建议
    /// 
    /// 基于当前集群状态提供优化建议
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
    /// use conflux::raft::validation::{ValidationConfig, ComprehensiveValidator};
    /// 
    /// let config = ValidationConfig::default();
    /// let validator = ComprehensiveValidator::new(config);
    /// 
    /// let suggestions = validator.get_cluster_suggestions(4, 100, 300, 10);
    /// println!("Cluster suggestions: {:?}", suggestions);
    /// ```
    pub fn get_cluster_suggestions(
        &self,
        current_cluster_size: usize,
        current_heartbeat: u64,
        current_election_min: u64,
        network_latency_ms: u64,
    ) -> ClusterSuggestions {
        debug!("Generating cluster suggestions");

        let mut suggestions = ClusterSuggestions::default();

        // 检查集群大小奇偶性
        if !self.cluster_validator.validate_cluster_parity(current_cluster_size) {
            suggestions.size_recommendations.push(format!(
                "Consider using odd cluster size instead of {} for better split-brain prevention",
                current_cluster_size
            ));
        }

        // 检查容错能力
        let fault_tolerance = self.cluster_validator.calculate_fault_tolerance(current_cluster_size);
        suggestions.fault_tolerance_info = format!(
            "Current cluster can tolerate {} node failures",
            fault_tolerance
        );

        // 检查超时配置
        let (recommended_heartbeat, recommended_min, _recommended_max) =
            self.timeout_validator.recommend_timeouts(network_latency_ms);

        if current_heartbeat != recommended_heartbeat {
            suggestions.timeout_recommendations.push(format!(
                "Consider adjusting heartbeat interval from {}ms to {}ms for {}ms network latency",
                current_heartbeat, recommended_heartbeat, network_latency_ms
            ));
        }

        if current_election_min != recommended_min {
            suggestions.timeout_recommendations.push(format!(
                "Consider adjusting election timeout min from {}ms to {}ms",
                current_election_min, recommended_min
            ));
        }

        // 网络配置建议
        if self.config.allow_localhost && self.config.allow_private_ips {
            suggestions.network_recommendations.push(
                "Consider disabling localhost and private IPs for production deployment".to_string()
            );
        }

        debug!("Generated {} suggestions", 
               suggestions.size_recommendations.len() + 
               suggestions.timeout_recommendations.len() + 
               suggestions.network_recommendations.len());

        suggestions
    }

    /// 获取验证配置
    /// 
    /// # Returns
    /// 
    /// 返回当前的验证配置
    pub fn get_config(&self) -> &ValidationConfig {
        &self.config
    }

    /// 更新验证配置
    /// 
    /// # Arguments
    /// 
    /// * `new_config` - 新的验证配置
    pub fn update_config(&mut self, new_config: ValidationConfig) {
        self.config = new_config;
        // 注意：这里需要重新创建验证器，但由于生命周期问题，
        // 在实际实现中可能需要不同的设计
    }
}

/// 集群优化建议
/// 
/// 包含各种类型的集群优化建议
#[derive(Debug, Default)]
pub struct ClusterSuggestions {
    /// 集群大小相关建议
    pub size_recommendations: Vec<String>,
    /// 超时配置相关建议
    pub timeout_recommendations: Vec<String>,
    /// 网络配置相关建议
    pub network_recommendations: Vec<String>,
    /// 容错能力信息
    pub fault_tolerance_info: String,
}

impl ClusterSuggestions {
    /// 检查是否有任何建议
    /// 
    /// # Returns
    /// 
    /// 如果有建议返回true，否则返回false
    pub fn has_suggestions(&self) -> bool {
        !self.size_recommendations.is_empty() ||
        !self.timeout_recommendations.is_empty() ||
        !self.network_recommendations.is_empty()
    }

    /// 获取所有建议的总数
    /// 
    /// # Returns
    /// 
    /// 返回建议总数
    pub fn total_suggestions(&self) -> usize {
        self.size_recommendations.len() +
        self.timeout_recommendations.len() +
        self.network_recommendations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_add_node() {
        let config = ValidationConfig::default();
        let validator = ComprehensiveValidator::new(config);
        let existing_nodes = vec![(1, "127.0.0.1:8080".to_string())];

        // Valid addition
        let result = validator.validate_add_node(2, "127.0.0.1:8081", &existing_nodes);
        assert!(result.is_ok());

        // Duplicate node ID
        let result = validator.validate_add_node(1, "127.0.0.1:8082", &existing_nodes);
        assert!(result.is_err());

        // Duplicate address
        let result = validator.validate_add_node(3, "127.0.0.1:8080", &existing_nodes);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_remove_node() {
        let config = ValidationConfig::default();
        let validator = ComprehensiveValidator::new(config);
        let existing_nodes = vec![(1, "127.0.0.1:8080".to_string()), (2, "127.0.0.1:8081".to_string())];

        // Valid removal
        let result = validator.validate_remove_node(2, &existing_nodes);
        assert!(result.is_ok());

        // Non-existent node
        let result = validator.validate_remove_node(3, &existing_nodes);
        assert!(result.is_err());

        // Cannot remove last node
        let single_node = vec![(1, "127.0.0.1:8080".to_string())];
        let result = validator.validate_remove_node(1, &single_node);
        assert!(result.is_err());
    }

    #[test]
    fn test_cluster_suggestions() {
        let config = ValidationConfig::default();
        let validator = ComprehensiveValidator::new(config);

        let suggestions = validator.get_cluster_suggestions(4, 100, 300, 10);
        
        // Should suggest odd cluster size
        assert!(suggestions.has_suggestions());
        assert!(suggestions.size_recommendations.iter().any(|s| s.contains("odd cluster size")));
    }
}
