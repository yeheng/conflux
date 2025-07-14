//! 集群验证模块
//!
//! 提供集群大小和成员管理的验证功能

use super::config::ValidationConfig;
use crate::error::{ConfluxError, Result};
use crate::raft::types::NodeId;
use std::sync::Arc;
use tracing::debug;

/// 集群验证器
///
/// 专门负责集群大小、成员管理等集群级别的验证
pub struct ClusterValidator {
    config: Arc<ValidationConfig>,
}

impl ClusterValidator {
    /// 创建新的集群验证器
    ///
    /// # Arguments
    ///
    /// * `config` - 验证配置
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, ClusterValidator};
    /// use std::sync::Arc;
    ///
    /// let config = ValidationConfig::default();
    /// let validator = ClusterValidator::new(Arc::new(config));
    /// ```
    pub fn new(config: Arc<ValidationConfig>) -> Self {
        Self { config }
    }

    /// 验证集群大小
    ///
    /// 检查添加新节点后的集群大小是否超过限制
    ///
    /// # Arguments
    ///
    /// * `current_size` - 当前集群大小
    /// * `adding_nodes` - 要添加的节点数量
    ///
    /// # Returns
    ///
    /// 如果集群大小合理返回Ok(())，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, ClusterValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = ClusterValidator::new(&config);
    ///
    /// assert!(validator.validate_cluster_size(5, 1).is_ok());
    /// assert!(validator.validate_cluster_size(100, 1).is_err()); // 超过默认限制
    /// ```
    pub fn validate_cluster_size(&self, current_size: usize, adding_nodes: usize) -> Result<()> {

        if current_size > self.config.max_cluster_size {
            return Err(ConfluxError::validation(format!(
                "Cluster size would exceed maximum: {} + {} > {}",
                current_size, adding_nodes, self.config.max_cluster_size
            )));
        }
        let new_size = current_size + adding_nodes;

        debug!(
            "Validating cluster size: current={}, adding={}, new={}",
            current_size, adding_nodes, new_size
        );

        if new_size > self.config.max_cluster_size {
            return Err(ConfluxError::validation(format!(
                "Cluster size would exceed maximum: {} > {}",
                new_size, self.config.max_cluster_size
            )));
        }

        debug!("Cluster size {} is valid", new_size);
        Ok(())
    }

    /// 验证集群最小大小
    ///
    /// 检查移除节点后集群是否还能正常运行
    ///
    /// # Arguments
    ///
    /// * `current_size` - 当前集群大小
    /// * `removing_nodes` - 要移除的节点数量
    ///
    /// # Returns
    ///
    /// 如果移除后集群仍可运行返回Ok(())，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, ClusterValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = ClusterValidator::new(&config);
    ///
    /// assert!(validator.validate_minimum_cluster_size(3, 1).is_ok());
    /// assert!(validator.validate_minimum_cluster_size(1, 1).is_err()); // 不能移除最后一个节点
    /// ```
    pub fn validate_minimum_cluster_size(
        &self,
        current_size: usize,
        removing_nodes: usize,
    ) -> Result<()> {
        debug!(
            "Validating minimum cluster size: current={}, removing={}",
            current_size, removing_nodes
        );

        if removing_nodes >= current_size {
            return Err(ConfluxError::validation(
                "Cannot remove all nodes from cluster".to_string(),
            ));
        }

        let remaining_size = current_size - removing_nodes;
        if remaining_size == 0 {
            return Err(ConfluxError::validation(
                "Cannot remove the last node from cluster".to_string(),
            ));
        }

        debug!(
            "Minimum cluster size validation passed: {} nodes remaining",
            remaining_size
        );
        Ok(())
    }

    /// 验证集群奇偶性
    ///
    /// 检查集群大小是否有利于Raft共识（奇数节点更好）
    ///
    /// # Arguments
    ///
    /// * `cluster_size` - 集群大小
    ///
    /// # Returns
    ///
    /// 返回是否为奇数大小的集群
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, ClusterValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = ClusterValidator::new(&config);
    ///
    /// assert!(validator.validate_cluster_parity(3)); // 奇数，推荐
    /// assert!(!validator.validate_cluster_parity(4)); // 偶数，不推荐但允许
    /// ```
    pub fn validate_cluster_parity(&self, cluster_size: usize) -> bool {
        debug!("Validating cluster parity for size: {}", cluster_size);

        let is_odd = cluster_size % 2 == 1;
        if !is_odd {
            debug!(
                "Warning: Even cluster size {} may lead to split-brain scenarios",
                cluster_size
            );
        }

        is_odd
    }

    /// 验证节点在集群中的存在性
    ///
    /// 检查指定节点是否存在于集群中
    ///
    /// # Arguments
    ///
    /// * `node_id` - 要检查的节点ID
    /// * `existing_nodes` - 现有节点列表
    ///
    /// # Returns
    ///
    /// 如果节点存在返回Ok(())，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, ClusterValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = ClusterValidator::new(&config);
    /// let existing_nodes = vec![(1, "127.0.0.1:8080".to_string()), (2, "127.0.0.1:8081".to_string())];
    ///
    /// assert!(validator.validate_node_exists(1, &existing_nodes).is_ok());
    /// assert!(validator.validate_node_exists(3, &existing_nodes).is_err());
    /// ```
    pub fn validate_node_exists(
        &self,
        node_id: NodeId,
        existing_nodes: &[(NodeId, String)],
    ) -> Result<()> {
        debug!(
            "Validating node {} exists in cluster of {} nodes",
            node_id,
            existing_nodes.len()
        );

        if !existing_nodes.iter().any(|(id, _)| *id == node_id) {
            return Err(ConfluxError::validation(format!(
                "Node ID {} does not exist in cluster",
                node_id
            )));
        }

        debug!("Node {} exists in cluster", node_id);
        Ok(())
    }

    /// 验证集群健康状态
    ///
    /// 检查集群是否有足够的节点来维持可用性
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
    /// use conflux::raft::validation::{ValidationConfig, ClusterValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = ClusterValidator::new(&config);
    ///
    /// assert!(validator.validate_cluster_health(5, 3).is_ok()); // 大多数节点健康
    /// assert!(validator.validate_cluster_health(5, 2).is_err()); // 少数节点健康
    /// ```
    pub fn validate_cluster_health(&self, total_nodes: usize, healthy_nodes: usize) -> Result<()> {
        debug!(
            "Validating cluster health: {}/{} nodes healthy",
            healthy_nodes, total_nodes
        );

        if healthy_nodes == 0 {
            return Err(ConfluxError::validation(
                "No healthy nodes in cluster".to_string(),
            ));
        }

        if total_nodes == 0 {
            return Err(ConfluxError::validation("Empty cluster".to_string()));
        }

        if healthy_nodes > total_nodes {
            return Err(ConfluxError::validation(
                "Healthy nodes cannot exceed total nodes".to_string(),
            ));
        }

        // For Raft consensus, we need a majority of nodes to be healthy
        let required_majority = (total_nodes / 2) + 1;
        if healthy_nodes < required_majority {
            return Err(ConfluxError::validation(format!(
                "Insufficient healthy nodes for consensus: {}/{} (need {})",
                healthy_nodes, total_nodes, required_majority
            )));
        }

        debug!("Cluster health validation passed");
        Ok(())
    }

    /// 计算集群的容错能力
    ///
    /// 计算集群可以容忍多少个节点故障
    ///
    /// # Arguments
    ///
    /// * `cluster_size` - 集群大小
    ///
    /// # Returns
    ///
    /// 返回可容忍的故障节点数量
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, ClusterValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = ClusterValidator::new(&config);
    ///
    /// assert_eq!(validator.calculate_fault_tolerance(5), 2); // 5节点集群可容忍2个故障
    /// assert_eq!(validator.calculate_fault_tolerance(3), 1); // 3节点集群可容忍1个故障
    /// ```
    pub fn calculate_fault_tolerance(&self, cluster_size: usize) -> usize {
        if cluster_size == 0 {
            return 0;
        }

        // Raft can tolerate (n-1)/2 failures where n is cluster size
        let fault_tolerance = (cluster_size - 1) / 2;

        debug!(
            "Cluster size {} can tolerate {} node failures",
            cluster_size, fault_tolerance
        );

        fault_tolerance
    }

    /// 推荐的集群大小
    ///
    /// 根据期望的容错能力推荐集群大小
    ///
    /// # Arguments
    ///
    /// * `desired_fault_tolerance` - 期望的容错节点数
    ///
    /// # Returns
    ///
    /// 返回推荐的集群大小
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::{ValidationConfig, ClusterValidator};
    ///
    /// let config = ValidationConfig::default();
    /// let validator = ClusterValidator::new(&config);
    ///
    /// assert_eq!(validator.recommend_cluster_size(1), 3); // 容忍1个故障需要3个节点
    /// assert_eq!(validator.recommend_cluster_size(2), 5); // 容忍2个故障需要5个节点
    /// ```
    pub fn recommend_cluster_size(&self, desired_fault_tolerance: usize) -> usize {
        // For Raft, to tolerate f failures, we need 2f+1 nodes
        let recommended_size = 2 * desired_fault_tolerance + 1;

        debug!(
            "To tolerate {} failures, recommend cluster size of {}",
            desired_fault_tolerance, recommended_size
        );

        recommended_size
    }

    /// 获取验证配置
    pub fn config(&self) -> &Arc<ValidationConfig> {
        &self.config
    }
}
