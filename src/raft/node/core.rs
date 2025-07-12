//! Raft节点核心实现模块
//!
//! 包含RaftNode的主要实现，负责节点的创建、启动、停止和基本操作

use super::config::NodeConfig;
use super::resource_limiter::{ResourceLimiter, ResourceStats};
use crate::config::AppConfig;
use crate::error::Result;
use crate::raft::{
    auth::RaftAuthzService,
    metrics::RaftMetricsCollector,
    network::ConfluxNetworkFactory,
    store::{StateMachineManager, Store},
    types::*,
    validation::RaftInputValidator,
};
use openraft::Raft;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// Raft节点核心实现
///
/// 集成了openraft::Raft实例的Raft节点，提供完整的分布式共识功能
///
/// # Examples
///
/// ```rust
/// use conflux::raft::node::{RaftNode, NodeConfig};
/// use conflux::config::AppConfig;
///
/// # tokio_test::block_on(async {
/// let config = NodeConfig::default();
/// let app_config = AppConfig::default();
/// let mut node = RaftNode::new(config, &app_config).await.unwrap();
///
/// // 启动节点
/// node.start().await.unwrap();
///
/// // 检查是否为领导者
/// let is_leader = node.is_leader().await;
/// println!("Is leader: {}", is_leader);
/// # });
/// ```
pub struct RaftNode {
    /// 节点配置
    config: NodeConfig,
    /// 存储实例
    store: Arc<Store>,
    /// 网络工厂
    network_factory: Arc<RwLock<ConfluxNetworkFactory>>,
    /// 当前集群成员
    members: Arc<RwLock<BTreeSet<NodeId>>>,
    /// 实际的Raft实例
    raft: Option<ConfluxRaft>,
    /// 状态机管理器句柄
    state_machine_handle: Option<tokio::task::JoinHandle<()>>,
    /// 指标收集器
    metrics_collector: Arc<RaftMetricsCollector>,
    /// 客户端请求资源限制器
    resource_limiter: Arc<ResourceLimiter>,
    /// 可选的集群操作授权服务
    authz_service: Option<Arc<RaftAuthzService>>,
    /// 集群操作输入验证器
    input_validator: Arc<RaftInputValidator>,
}

impl RaftNode {
    /// 创建新的Raft节点
    ///
    /// # Arguments
    ///
    /// * `config` - 节点配置
    /// * `app_config` - 应用程序配置
    ///
    /// # Returns
    ///
    /// 返回创建的RaftNode实例
    ///
    /// # Errors
    ///
    /// 如果存储初始化失败或其他组件创建失败，返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::node::{RaftNode, NodeConfig};
    /// use conflux::config::AppConfig;
    ///
    /// # tokio_test::block_on(async {
    /// let config = NodeConfig::default();
    /// let app_config = AppConfig::default();
    /// let node = RaftNode::new(config, &app_config).await.unwrap();
    /// # });
    /// ```
    pub async fn new(config: NodeConfig, app_config: &AppConfig) -> Result<Self> {
        info!(
            "Creating Raft node {} at {}",
            config.node_id, config.address
        );

        // 创建存储并获取事件接收器
        let (store, event_receiver) = Store::new(&app_config.storage.data_dir).await?;
        let store = Arc::new(store);

        // 启动状态机管理器
        let mut state_machine_manager = StateMachineManager::new(store.clone(), event_receiver);
        let state_machine_handle = tokio::spawn(async move {
            state_machine_manager.run().await;
        });

        // 创建网络工厂
        let network_factory = Arc::new(RwLock::new(ConfluxNetworkFactory::new(
            config.network_config.clone(),
        )));

        // 初始化成员列表（包含自己）
        let mut members = BTreeSet::new();
        members.insert(config.node_id);

        // 创建指标收集器
        let metrics_collector = Arc::new(RaftMetricsCollector::new(config.node_id));

        // 创建资源限制器
        let resource_limiter = Arc::new(ResourceLimiter::new(config.resource_limits.clone()));

        // 创建输入验证器
        let input_validator = Arc::new(RaftInputValidator::new());

        Ok(Self {
            config,
            store,
            network_factory,
            members: Arc::new(RwLock::new(members)),
            raft: None, // 将在start()中初始化
            state_machine_handle: Some(state_machine_handle),
            metrics_collector,
            resource_limiter,
            authz_service: None, // 可以稍后通过set_authz_service()设置
            input_validator,
        })
    }

    /// 获取节点ID
    ///
    /// # Returns
    ///
    /// 返回节点的唯一标识符
    pub fn node_id(&self) -> NodeId {
        self.config.node_id
    }

    /// 获取节点地址
    ///
    /// # Returns
    ///
    /// 返回节点的网络地址
    pub fn address(&self) -> &str {
        &self.config.address
    }

    /// 获取存储实例
    ///
    /// # Returns
    ///
    /// 返回存储实例的Arc引用
    pub fn store(&self) -> Arc<Store> {
        self.store.clone()
    }

    /// 获取指标收集器
    ///
    /// # Returns
    ///
    /// 返回指标收集器的Arc引用
    pub fn metrics_collector(&self) -> Arc<RaftMetricsCollector> {
        self.metrics_collector.clone()
    }

    /// 获取资源限制器
    ///
    /// # Returns
    ///
    /// 返回资源限制器的Arc引用
    pub fn resource_limiter(&self) -> Arc<ResourceLimiter> {
        self.resource_limiter.clone()
    }

    /// 设置集群操作授权服务
    ///
    /// # Arguments
    ///
    /// * `authz_service` - 授权服务实例
    pub fn set_authz_service(&mut self, authz_service: Arc<RaftAuthzService>) {
        info!(
            "Setting authorization service for node {}",
            self.config.node_id
        );
        self.authz_service = Some(authz_service);
    }

    /// 获取授权服务
    ///
    /// # Returns
    ///
    /// 返回授权服务的可选Arc引用
    pub fn authz_service(&self) -> Option<Arc<RaftAuthzService>> {
        self.authz_service.clone()
    }

    /// 获取输入验证器
    ///
    /// # Returns
    ///
    /// 返回输入验证器的Arc引用
    pub fn input_validator(&self) -> Arc<RaftInputValidator> {
        self.input_validator.clone()
    }

    /// 启动节点并初始化Raft实例
    ///
    /// # Returns
    ///
    /// 如果启动成功返回Ok(())，否则返回错误
    ///
    /// # Errors
    ///
    /// 如果Raft实例初始化失败或集群初始化失败，返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::node::{RaftNode, NodeConfig};
    /// use conflux::config::AppConfig;
    ///
    /// # tokio_test::block_on(async {
    /// let config = NodeConfig::default();
    /// let app_config = AppConfig::default();
    /// let mut node = RaftNode::new(config, &app_config).await.unwrap();
    ///
    /// node.start().await.unwrap();
    /// # });
    /// ```
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Raft node {}", self.config.node_id);

        // openraft 0.9 正确初始化方式：直接用 Arc<Store> 作为 storage
        let network_factory = {
            let factory = self.network_factory.read().await;
            factory.clone()
        };

        let mut raft_config = self.config.raft_config.clone();
        raft_config.heartbeat_interval = self.config.heartbeat_interval;
        raft_config.election_timeout_min = self.config.election_timeout_min;
        raft_config.election_timeout_max = self.config.election_timeout_max;

        // openraft 0.9 storage v2 不再使用 Adaptor
        // 直接使用 Store 作为 RaftLogStorage 和创建 ConfluxStateMachineWrapper
        let log_storage = self.store.clone();
        let state_machine =
            crate::raft::state_machine::ConfluxStateMachineWrapper::new(self.store.clone());

        // openraft 0.9 Raft::new 需要5个参数：node_id, config, network_factory, log_storage, state_machine
        match Raft::new(
            self.config.node_id,
            Arc::new(raft_config),
            network_factory,
            log_storage,
            state_machine,
        )
        .await
        {
            Ok(raft) => {
                self.raft = Some(raft);
                info!(
                    "Raft instance initialized successfully for node {}",
                    self.config.node_id
                );
            }
            Err(e) => {
                error!("Failed to initialize Raft instance: {}", e);
                return Err(crate::error::ConfluxError::raft(format!(
                    "Raft initialization failed: {}",
                    e
                )));
            }
        }

        // 如果需要，初始化单节点集群
        if self.is_single_node_cluster().await {
            self.initialize_cluster().await?;
        }

        info!("Raft node {} started successfully", self.config.node_id);
        Ok(())
    }

    /// 获取Raft实例（如果可用）
    ///
    /// # Returns
    ///
    /// 返回Raft实例的可选引用
    pub fn get_raft(&self) -> Option<&ConfluxRaft> {
        self.raft.as_ref()
    }

    /// 通过Raft共识提交客户端写请求（带资源限制）
    ///
    /// # Arguments
    ///
    /// * `request` - 客户端请求
    ///
    /// # Returns
    ///
    /// 返回客户端写响应
    ///
    /// # Errors
    ///
    /// 如果资源限制检查失败、Raft未初始化或写操作失败，返回错误
    pub async fn client_write(&self, request: ClientRequest) -> Result<ClientWriteResponse> {
        let start_time = std::time::Instant::now();

        info!(
            "Processing client write through Raft consensus on node {}",
            self.config.node_id
        );

        // 计算请求大小（简化版 - 实际实现中会序列化）
        let request_size = std::mem::size_of_val(&request) + request.command.estimate_size();

        // 首先检查资源限制
        let _permit = self
            .resource_limiter
            .check_request_allowed(request_size, None) // TODO: 可用时添加客户端ID
            .await?;

        let result = if let Some(ref raft) = self.raft {
            // 始终通过Raft共识路由 - 无回退
            match raft.client_write(request).await {
                Ok(raft_response) => {
                    // raft_response.data 包含我们的 ClientWriteResponse
                    Ok(raft_response.data)
                }
                Err(e) => {
                    error!("Raft client write failed: {}", e);
                    Err(crate::error::ConfluxError::raft(format!(
                        "Raft write failed: {}",
                        e
                    )))
                }
            }
        } else {
            // 如果Raft未初始化则返回错误而不是回退
            Err(crate::error::ConfluxError::raft(
                "Raft not initialized - cannot process write requests",
            ))
        };

        // 记录请求指标
        let latency = start_time.elapsed();
        let success = result.is_ok();
        self.metrics_collector
            .record_request(latency, success)
            .await;

        result
    }

    /// 停止节点（占位符实现）
    ///
    /// # Returns
    ///
    /// 如果停止成功返回Ok(())，否则返回错误
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Raft node {}", self.config.node_id);
        debug!("Raft node {} stopped successfully", self.config.node_id);
        Ok(())
    }

    /// 获取当前集群成员
    ///
    /// # Returns
    ///
    /// 返回当前集群成员的集合
    pub async fn get_members(&self) -> BTreeSet<NodeId> {
        self.members.read().await.clone()
    }

    /// 获取资源使用统计信息
    ///
    /// # Returns
    ///
    /// 返回当前的资源使用统计
    pub fn get_resource_stats(&self) -> ResourceStats {
        self.resource_limiter.get_resource_stats()
    }

    /// 等待成为领导者
    ///
    /// # Arguments
    ///
    /// * `timeout` - 等待超时时间
    ///
    /// # Returns
    ///
    /// 如果在超时时间内成为领导者返回Ok(())，否则返回错误
    pub async fn wait_for_leadership(&self, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if self.is_leader().await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(crate::error::ConfluxError::raft(
            "Timeout waiting for leadership",
        ))
    }

    /// 检查这是否是单节点集群
    async fn is_single_node_cluster(&self) -> bool {
        let members = self.members.read().await;
        members.len() == 1 && members.contains(&self.config.node_id)
    }

    /// 检查此节点是否为领导者
    ///
    /// # Returns
    ///
    /// 如果是领导者返回true，否则返回false
    pub async fn is_leader(&self) -> bool {
        if let Some(ref raft) = self.raft {
            // 使用实际的Raft实例检查领导权
            (raft.ensure_linearizable().await).is_ok()
        } else {
            false
        }
    }

    /// 获取当前领导者ID
    ///
    /// # Returns
    ///
    /// 返回当前领导者的节点ID，如果没有领导者则返回None
    pub async fn get_leader(&self) -> Option<NodeId> {
        if let Some(ref raft) = self.raft {
            // 使用实际的Raft指标获取领导者ID
            let metrics = raft.metrics().borrow().clone();
            metrics.current_leader
        } else {
            None
        }
    }

    /// 获取当前Raft指标
    ///
    /// # Returns
    ///
    /// 返回当前的Raft指标信息
    ///
    /// # Errors
    ///
    /// 如果Raft未初始化，返回错误
    pub async fn get_metrics(&self) -> Result<RaftMetrics> {
        if let Some(ref raft) = self.raft {
            // 从Raft实例获取真实指标
            let raft_metrics = raft.metrics().borrow().clone();

            // 从成员配置中提取成员节点ID
            let membership: BTreeSet<NodeId> = raft_metrics
                .membership_config
                .membership()
                .nodes()
                .map(|(id, _)| *id)
                .collect();

            // 使用最新数据更新指标收集器
            self.metrics_collector
                .update_node_metrics(
                    raft_metrics.current_term,
                    raft_metrics.last_log_index.unwrap_or(0),
                    raft_metrics.last_applied.map(|id| id.index).unwrap_or(0),
                    raft_metrics.current_leader,
                    self.is_leader().await,
                )
                .await;

            Ok(RaftMetrics {
                node_id: self.config.node_id,
                current_term: raft_metrics.current_term,
                last_log_index: raft_metrics.last_log_index.unwrap_or(0),
                last_applied: raft_metrics.last_applied.map(|id| id.index).unwrap_or(0),
                leader_id: raft_metrics.current_leader,
                membership,
                is_leader: self.is_leader().await,
            })
        } else {
            Err(crate::error::ConfluxError::raft("Raft not initialized"))
        }
    }

    /// 获取节点健康状态
    ///
    /// # Returns
    ///
    /// 返回节点的健康状态信息
    pub async fn get_node_health(&self) -> Result<crate::raft::metrics::NodeHealth> {
        Ok(self.metrics_collector.get_node_health().await)
    }

    /// 获取当前超时配置
    ///
    /// # Returns
    ///
    /// 返回(心跳间隔, 选举超时最小值, 选举超时最大值)的元组
    pub fn get_timeout_config(&self) -> (u64, u64, u64) {
        (
            self.config.heartbeat_interval,
            self.config.election_timeout_min,
            self.config.election_timeout_max,
        )
    }

    /// 更新超时配置（内部方法）
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
    pub(crate) fn update_timeout_config(
        &mut self,
        heartbeat_interval: Option<u64>,
        election_timeout_min: Option<u64>,
        election_timeout_max: Option<u64>,
    ) -> Result<()> {
        // 更新配置
        if let Some(interval) = heartbeat_interval {
            self.config.heartbeat_interval = interval;
        }
        if let Some(min_timeout) = election_timeout_min {
            self.config.election_timeout_min = min_timeout;
        }
        if let Some(max_timeout) = election_timeout_max {
            self.config.election_timeout_max = max_timeout;
        }

        // 验证超时范围
        if self.config.election_timeout_min >= self.config.election_timeout_max {
            return Err(crate::error::ConfluxError::raft(
                "Election timeout min must be less than max",
            ));
        }

        if self.config.heartbeat_interval >= self.config.election_timeout_min {
            return Err(crate::error::ConfluxError::raft(
                "Heartbeat interval must be less than election timeout min",
            ));
        }

        Ok(())
    }

    /// 初始化单节点集群
    async fn initialize_cluster(&self) -> Result<()> {
        if let Some(ref raft) = self.raft {
            info!(
                "Initializing single-node cluster for node {}",
                self.config.node_id
            );

            let mut members = BTreeSet::new();
            members.insert(self.config.node_id);

            raft.initialize(members).await.map_err(|e| {
                crate::error::ConfluxError::raft(format!("Failed to initialize cluster: {}", e))
            })?;

            info!("Single-node cluster initialized successfully");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StorageConfig;
    use tempfile::TempDir;

    fn create_test_app_config() -> AppConfig {
        let temp_dir = TempDir::new().unwrap();
        AppConfig {
            storage: StorageConfig {
                data_dir: temp_dir.path().to_string_lossy().to_string(),
                max_open_files: 1000,
                cache_size_mb: 256,
                write_buffer_size_mb: 64,
                max_write_buffer_number: 3,
            },
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_raft_node_creation() {
        let config = NodeConfig::default();
        let app_config = create_test_app_config();

        let node = RaftNode::new(config, &app_config).await.unwrap();

        assert_eq!(node.node_id(), 1);
        assert_eq!(node.address(), "127.0.0.1:8080");
        assert!(node.get_raft().is_none()); // Raft未启动
    }

    #[tokio::test]
    async fn test_raft_node_start() {
        let config = NodeConfig::default();
        let app_config = create_test_app_config();

        let mut node = RaftNode::new(config, &app_config).await.unwrap();

        // 启动节点
        node.start().await.unwrap();

        assert!(node.get_raft().is_some()); // Raft已启动
    }

    #[tokio::test]
    async fn test_resource_stats() {
        let config = NodeConfig::default();
        let app_config = create_test_app_config();

        let node = RaftNode::new(config, &app_config).await.unwrap();
        let stats = node.get_resource_stats();

        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.rejected_requests, 0);
    }
}
