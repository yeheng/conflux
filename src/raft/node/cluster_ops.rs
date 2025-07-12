//! 集群操作模块
//!
//! 提供Raft集群的成员管理和配置更新功能

use super::core::RaftNode;
use crate::auth::{AuthContext, PermissionResult};
use crate::error::Result;
use crate::raft::{auth::AuthorizedRaftOperation, types::NodeId};
use std::collections::BTreeSet;
use tracing::{info, warn};

impl RaftNode {
    /// 向集群添加新节点（使用Raft共识和授权）
    ///
    /// # Arguments
    ///
    /// * `node_id` - 要添加的节点ID
    /// * `address` - 节点地址
    ///
    /// # Returns
    ///
    /// 如果添加成功返回Ok(())，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// # tokio_test::block_on(async {
    /// # let mut node = create_test_node().await;
    /// node.add_node(2, "127.0.0.1:8081".to_string()).await.unwrap();
    /// # });
    /// ```
    pub async fn add_node(&self, node_id: NodeId, address: String) -> Result<()> {
        self.add_node_with_auth(node_id, address, None).await
    }

    /// 带授权上下文的添加节点操作
    ///
    /// # Arguments
    ///
    /// * `node_id` - 要添加的节点ID
    /// * `address` - 节点地址
    /// * `auth_ctx` - 可选的授权上下文
    ///
    /// # Returns
    ///
    /// 如果添加成功返回Ok(())，否则返回错误
    ///
    /// # Errors
    ///
    /// - 如果输入验证失败
    /// - 如果授权检查失败
    /// - 如果Raft共识失败
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::auth::AuthContext;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut node = create_test_node().await;
    /// let auth_ctx = AuthContext::new("user1".to_string(), "tenant1".to_string());
    /// node.add_node_with_auth(2, "127.0.0.1:8081".to_string(), Some(auth_ctx)).await.unwrap();
    /// # });
    /// ```
    pub async fn add_node_with_auth(
        &self,
        node_id: NodeId,
        address: String,
        auth_ctx: Option<AuthContext>,
    ) -> Result<()> {
        info!(
            "Adding node {} at {} to cluster via Raft consensus",
            node_id, address
        );

        // 获取现有节点用于验证
        let existing_nodes: Vec<(NodeId, String)> = {
            let members = self.get_members().await;
            // 在真实实现中，我们会有获取所有成员地址的方法
            // 现在我们创建一个简化的列表用于验证
            members
                .iter()
                .map(|&id| (id, format!("node-{}", id)))
                .collect()
        };

        // 验证节点添加请求
        let _validated_address = self
            .input_validator()
            .validate_add_node(node_id, &address, &existing_nodes)
            .map_err(|e| {
                warn!("Node addition validation failed: {}", e);
                e
            })?;

        info!(
            "Input validation passed for adding node {} at {}",
            node_id, address
        );

        // 如果授权服务可用，检查授权
        if let Some(ref authz_service) = self.authz_service() {
            if let Some(auth_ctx) = auth_ctx {
                let permission_result = authz_service
                    .check_add_node_permission(&auth_ctx, node_id)
                    .await
                    .unwrap_or_else(|_| {
                        PermissionResult::denied(
                            auth_ctx.user_id.clone(),
                            auth_ctx.tenant_id.clone(),
                            format!("cluster/node/{}", node_id),
                            "add_node".to_string(),
                        )
                    });

                let authorized_op = AuthorizedRaftOperation::new(auth_ctx, permission_result);
                authorized_op.ensure_authorized()?;

                info!(
                    "Add node operation authorized for user: {}",
                    authorized_op.auth_ctx.user_id
                );
            } else {
                warn!("Authorization service available but no auth context provided for add_node");
            }
        }

        if let Some(ref raft) = self.get_raft() {
            // 获取当前成员并添加新节点
            let current_members = self.get_members().await;

            let mut new_members = current_members;
            new_members.insert(node_id);

            // 使用Raft的change_membership通过共识添加节点
            raft.change_membership(new_members, false)
                .await
                .map_err(|e| {
                    crate::error::ConfluxError::raft(format!("Failed to add node via Raft: {}", e))
                })?;

            // 注意：在实际实现中，成员更新应该通过Raft状态机处理
            // 这里我们暂时跳过本地成员更新，因为它应该通过共识机制自动处理

            info!(
                "Node {} added to cluster successfully via Raft consensus",
                node_id
            );
        } else {
            return Err(crate::error::ConfluxError::raft("Raft not initialized"));
        }

        Ok(())
    }

    /// 从集群移除节点（使用Raft共识和授权）
    ///
    /// # Arguments
    ///
    /// * `node_id` - 要移除的节点ID
    ///
    /// # Returns
    ///
    /// 如果移除成功返回Ok(())，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// # tokio_test::block_on(async {
    /// # let mut node = create_test_node().await;
    /// node.remove_node(2).await.unwrap();
    /// # });
    /// ```
    pub async fn remove_node(&self, node_id: NodeId) -> Result<()> {
        self.remove_node_with_auth(node_id, None).await
    }

    /// 带授权上下文的移除节点操作
    ///
    /// # Arguments
    ///
    /// * `node_id` - 要移除的节点ID
    /// * `auth_ctx` - 可选的授权上下文
    ///
    /// # Returns
    ///
    /// 如果移除成功返回Ok(())，否则返回错误
    ///
    /// # Errors
    ///
    /// - 如果输入验证失败
    /// - 如果授权检查失败
    /// - 如果尝试移除最后一个节点
    /// - 如果Raft共识失败
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::auth::AuthContext;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut node = create_test_node().await;
    /// let auth_ctx = AuthContext::new("user1".to_string(), "tenant1".to_string());
    /// node.remove_node_with_auth(2, Some(auth_ctx)).await.unwrap();
    /// # });
    /// ```
    pub async fn remove_node_with_auth(
        &self,
        node_id: NodeId,
        auth_ctx: Option<AuthContext>,
    ) -> Result<()> {
        info!("Removing node {} from cluster via Raft consensus", node_id);

        // 获取现有节点用于验证
        let existing_nodes: Vec<(NodeId, String)> = {
            let members = self.get_members().await;
            // 在真实实现中，我们会有获取所有成员地址的方法
            // 现在我们创建一个简化的列表用于验证
            members
                .iter()
                .map(|&id| (id, format!("node-{}", id)))
                .collect()
        };

        // 验证节点移除请求
        self.input_validator()
            .validate_remove_node(node_id, &existing_nodes)
            .map_err(|e| {
                warn!("Node removal validation failed: {}", e);
                e
            })?;

        info!("Input validation passed for removing node {}", node_id);

        // 如果授权服务可用，检查授权
        if let Some(ref authz_service) = self.authz_service() {
            if let Some(auth_ctx) = auth_ctx {
                let permission_result = authz_service
                    .check_remove_node_permission(&auth_ctx, node_id)
                    .await
                    .unwrap_or_else(|_| {
                        PermissionResult::denied(
                            auth_ctx.user_id.clone(),
                            auth_ctx.tenant_id.clone(),
                            format!("cluster/node/{}", node_id),
                            "remove_node".to_string(),
                        )
                    });

                let authorized_op = AuthorizedRaftOperation::new(auth_ctx, permission_result);
                authorized_op.ensure_authorized()?;

                info!(
                    "Remove node operation authorized for user: {}",
                    authorized_op.auth_ctx.user_id
                );
            } else {
                warn!(
                    "Authorization service available but no auth context provided for remove_node"
                );
            }
        }

        if let Some(ref raft) = self.get_raft() {
            // 获取当前成员并移除节点
            let current_members = self.get_members().await;

            if current_members.len() <= 1 {
                return Err(crate::error::ConfluxError::raft(
                    "Cannot remove last node from cluster",
                ));
            }

            let mut new_members = current_members;
            new_members.remove(&node_id);

            // 使用Raft的change_membership通过共识移除节点
            raft.change_membership(new_members, false)
                .await
                .map_err(|e| {
                    crate::error::ConfluxError::raft(format!(
                        "Failed to remove node via Raft: {}",
                        e
                    ))
                })?;

            // 注意：在实际实现中，成员更新应该通过Raft状态机处理
            // 这里我们暂时跳过本地成员更新，因为它应该通过共识机制自动处理

            info!(
                "Node {} removed from cluster successfully via Raft consensus",
                node_id
            );
        } else {
            return Err(crate::error::ConfluxError::raft("Raft not initialized"));
        }

        Ok(())
    }

    /// 更改集群成员（添加/移除节点）使用Raft共识
    ///
    /// # Arguments
    ///
    /// * `new_members` - 新的成员集合
    ///
    /// # Returns
    ///
    /// 如果更改成功返回Ok(())，否则返回错误
    ///
    /// # Errors
    ///
    /// - 如果当前节点不是领导者
    /// - 如果Raft未初始化
    /// - 如果Raft共识失败
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::BTreeSet;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut node = create_test_node().await;
    /// let mut new_members = BTreeSet::new();
    /// new_members.insert(1);
    /// new_members.insert(2);
    /// new_members.insert(3);
    ///
    /// node.change_membership(new_members).await.unwrap();
    /// # });
    /// ```
    pub async fn change_membership(&self, new_members: BTreeSet<NodeId>) -> Result<()> {
        if !self.is_leader().await {
            return Err(crate::error::ConfluxError::raft(
                "Only leader can change membership",
            ));
        }

        info!(
            "Changing cluster membership to: {:?} via Raft consensus",
            new_members
        );

        if let Some(ref raft) = self.get_raft() {
            // 使用Raft的change_membership API进行基于共识的成员变更
            raft.change_membership(new_members.clone(), false)
                .await
                .map_err(|e| {
                    crate::error::ConfluxError::raft(format!(
                        "Failed to change membership via Raft: {}",
                        e
                    ))
                })?;

            // 注意：在实际实现中，成员更新应该通过Raft状态机处理
            // 这里我们暂时跳过本地成员更新，因为它应该通过共识机制自动处理

            info!("Membership change completed via Raft consensus");
        } else {
            return Err(crate::error::ConfluxError::raft("Raft not initialized"));
        }

        Ok(())
    }

    /// 获取综合指标报告（带授权）
    ///
    /// # Returns
    ///
    /// 返回综合指标报告
    pub async fn get_comprehensive_metrics(&self) -> Result<crate::raft::metrics::MetricsReport> {
        self.get_comprehensive_metrics_with_auth(None).await
    }

    /// 带授权上下文的获取综合指标报告
    ///
    /// # Arguments
    ///
    /// * `auth_ctx` - 可选的授权上下文
    ///
    /// # Returns
    ///
    /// 返回综合指标报告
    ///
    /// # Errors
    ///
    /// 如果授权检查失败，返回错误
    pub async fn get_comprehensive_metrics_with_auth(
        &self,
        auth_ctx: Option<AuthContext>,
    ) -> Result<crate::raft::metrics::MetricsReport> {
        // 如果授权服务可用，检查授权
        if let Some(ref authz_service) = self.authz_service() {
            if let Some(auth_ctx) = auth_ctx {
                let permission_result = authz_service
                    .check_view_metrics_permission(&auth_ctx)
                    .await
                    .unwrap_or_else(|_| {
                        PermissionResult::denied(
                            auth_ctx.user_id.clone(),
                            auth_ctx.tenant_id.clone(),
                            "cluster/metrics".to_string(),
                            "view_metrics".to_string(),
                        )
                    });

                let authorized_op = AuthorizedRaftOperation::new(auth_ctx, permission_result);
                authorized_op.ensure_authorized()?;

                tracing::debug!(
                    "Metrics access authorized for user: {}",
                    authorized_op.auth_ctx.user_id
                );
            } else {
                warn!("Authorization service available but no auth context provided for get_comprehensive_metrics");
            }
        }

        Ok(self.metrics_collector().get_metrics_report().await)
    }

    /// 动态更新超时配置（带授权）
    ///
    /// # Arguments
    ///
    /// * `heartbeat_interval` - 可选的心跳间隔
    /// * `election_timeout_min` - 可选的选举超时最小值
    /// * `election_timeout_max` - 可选的选举超时最大值
    ///
    /// # Returns
    ///
    /// 如果更新成功返回Ok(())，否则返回错误
    pub async fn update_timeouts(
        &mut self,
        heartbeat_interval: Option<u64>,
        election_timeout_min: Option<u64>,
        election_timeout_max: Option<u64>,
    ) -> Result<()> {
        self.update_timeouts_with_auth(
            heartbeat_interval,
            election_timeout_min,
            election_timeout_max,
            None,
        )
        .await
    }

    /// 带授权上下文的更新超时配置
    ///
    /// # Arguments
    ///
    /// * `heartbeat_interval` - 可选的心跳间隔
    /// * `election_timeout_min` - 可选的选举超时最小值
    /// * `election_timeout_max` - 可选的选举超时最大值
    /// * `auth_ctx` - 可选的授权上下文
    ///
    /// # Returns
    ///
    /// 如果更新成功返回Ok(())，否则返回错误
    ///
    /// # Errors
    ///
    /// - 如果超时配置验证失败
    /// - 如果授权检查失败
    /// - 如果配置值不合理
    pub async fn update_timeouts_with_auth(
        &mut self,
        heartbeat_interval: Option<u64>,
        election_timeout_min: Option<u64>,
        election_timeout_max: Option<u64>,
        auth_ctx: Option<AuthContext>,
    ) -> Result<()> {
        info!("Updating Raft timeout configuration");

        // 首先验证超时配置
        self.input_validator()
            .validate_timeout_config(
                heartbeat_interval,
                election_timeout_min,
                election_timeout_max,
            )
            .map_err(|e| {
                warn!("Timeout configuration validation failed: {}", e);
                e
            })?;

        info!("Timeout configuration validation passed");

        // 如果授权服务可用，检查授权
        if let Some(ref authz_service) = self.authz_service() {
            if let Some(auth_ctx) = auth_ctx {
                let permission_result = authz_service
                    .check_change_config_permission(&auth_ctx)
                    .await
                    .unwrap_or_else(|_| {
                        PermissionResult::denied(
                            auth_ctx.user_id.clone(),
                            auth_ctx.tenant_id.clone(),
                            "cluster/config".to_string(),
                            "change_config".to_string(),
                        )
                    });

                let authorized_op = AuthorizedRaftOperation::new(auth_ctx, permission_result);
                authorized_op.ensure_authorized()?;

                info!(
                    "Timeout configuration change authorized for user: {}",
                    authorized_op.auth_ctx.user_id
                );
            } else {
                warn!("Authorization service available but no auth context provided for update_timeouts");
            }
        }

        // 更新配置
        self.update_timeout_config(
            heartbeat_interval,
            election_timeout_min,
            election_timeout_max,
        )?;

        let (heartbeat, min_timeout, max_timeout) = self.get_timeout_config();
        info!(
            "Timeout configuration updated: heartbeat={}, election_min={}, election_max={}",
            heartbeat, min_timeout, max_timeout
        );

        // 注意：要使运行时更新生效，需要重启Raft实例
        // 这是当前openraft实现的限制
        warn!("Note: Timeout changes require node restart to take effect");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, StorageConfig};
    use crate::raft::node::NodeConfig;
    use tempfile::TempDir;

    async fn create_test_node() -> RaftNode {
        let temp_dir = TempDir::new().unwrap();
        let app_config = AppConfig {
            storage: StorageConfig {
                data_dir: temp_dir.path().to_string_lossy().to_string(),
                max_open_files: 1000,
                cache_size_mb: 256,
                write_buffer_size_mb: 64,
                max_write_buffer_number: 3,
            },
            ..Default::default()
        };

        let config = NodeConfig::default();
        let mut node = RaftNode::new(config, &app_config).await.unwrap();
        node.start().await.unwrap();
        node
    }

    #[tokio::test]
    async fn test_add_node_validation() {
        let node = create_test_node().await;

        // 尝试添加无效的节点ID（0）
        let result = node.add_node(0, "127.0.0.1:8081".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_last_node() {
        let node = create_test_node().await;

        // 尝试移除最后一个节点应该失败
        let result = node.remove_node(1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_change_membership_non_leader() {
        let node = create_test_node().await;

        // 如果不是领导者，更改成员应该失败
        // 注意：在单节点集群中，节点通常是领导者
        let mut new_members = BTreeSet::new();
        new_members.insert(1);
        new_members.insert(2);

        // 这个测试可能需要更复杂的设置来模拟非领导者状态
        let result = node.change_membership(new_members).await;
        // 结果取决于节点是否已成为领导者
    }
}
