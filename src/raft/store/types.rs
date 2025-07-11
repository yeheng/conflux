use crate::raft::types::*;
use openraft::{storage::SnapshotMeta, LogId, StoredMembership, Vote};
use rocksdb::DB;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};

/// Store with RocksDB backend implementing RaftLogStorage
#[derive(Clone, Debug)]
pub struct Store {
    /// RocksDB instance for persistent storage
    pub(crate) db: Arc<DB>,

    /// In-memory cache for configurations
    pub(crate) configurations: Arc<RwLock<BTreeMap<ConfigKey, Config>>>,

    /// In-memory cache for configuration versions
    pub(crate) versions: Arc<RwLock<BTreeMap<u64, BTreeMap<u64, ConfigVersion>>>>,

    /// Name to config ID index
    pub(crate) name_index: Arc<RwLock<BTreeMap<ConfigKey, u64>>>,

    /// Next available config ID
    pub(crate) next_config_id: Arc<RwLock<u64>>,

    /// Change notification broadcaster
    pub(crate) change_notifier: Arc<broadcast::Sender<ConfigChangeEvent>>,

    /// Raft log storage (serialized as JSON strings like memstore)
    pub(crate) logs: Arc<RwLock<BTreeMap<u64, String>>>,

    /// Last purged log ID
    pub(crate) last_purged_log_id: Arc<RwLock<Option<LogId<NodeId>>>>,

    /// Vote storage
    pub(crate) vote: Arc<RwLock<Option<Vote<NodeId>>>>,

    /// 移除循环依赖：不再直接包含状态机
    /// 改为使用事件通信机制
    /// state_machine: Arc<RwLock<ConfluxStateMachine>>, // 已移除

    /// Current snapshot
    pub(crate) current_snapshot: Arc<RwLock<Option<ConfluxSnapshot>>>,

    /// Snapshot index counter
    pub(crate) snapshot_idx: Arc<Mutex<u64>>,

    /// 事件发送器，用于与状态机通信
    pub(crate) event_sender: Option<mpsc::Sender<StateChangeEvent>>,
}

/// 状态机管理器，负责处理状态变更事件循环
#[derive(Debug)]
pub struct StateMachineManager {
    /// Store实例用于处理状态变更
    store: Arc<Store>,
    /// 事件接收器
    event_receiver: mpsc::Receiver<StateChangeEvent>,
    /// 状态机实例
    state: crate::raft::state_machine::ConfluxStateMachine,
}

impl StateMachineManager {
    /// 创建新的状态机管理器
    pub fn new(store: Arc<Store>, event_receiver: mpsc::Receiver<StateChangeEvent>) -> Self {
        Self {
            state: crate::raft::state_machine::ConfluxStateMachine::new(store.clone()),
            store,
            event_receiver,
        }
    }

    /// 运行事件处理循环
    pub async fn run(&mut self) {
        while let Some(event) = self.event_receiver.recv().await {
            match event {
                StateChangeEvent::CommandApplied {
                    command,
                    response_sender,
                } => {
                    let result = self
                        .store
                        .apply_state_change(&command)
                        .await
                        .map_err(|e| format!("State change failed: {}", e));
                    let _ = response_sender.send(result);
                }
                StateChangeEvent::SnapshotRequest { response_sender } => {
                    let result = self
                        .state
                        .get_state()
                        .await
                        .map_err(|e| format!("Snapshot failed: {}", e));
                    let _ = response_sender.send(result);
                }
            }
        }
    }
}

/// 状态变更事件类型
#[derive(Debug)]
pub enum StateChangeEvent {
    /// 应用命令事件
    CommandApplied {
        command: RaftCommand,
        response_sender: oneshot::Sender<Result<ClientWriteResponse, String>>,
    },
    /// 快照请求事件
    SnapshotRequest {
        response_sender: oneshot::Sender<Result<Vec<u8>, String>>,
    },
}

/// State machine for Conflux
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfluxStateMachine {
    pub last_applied_log: Option<LogId<NodeId>>,
    pub last_membership: StoredMembership<NodeId, Node>,
    // Configuration data is stored in the main Store struct
}

/// Snapshot data for Conflux
#[derive(Debug)]
pub struct ConfluxSnapshot {
    pub meta: SnapshotMeta<NodeId, Node>,
    pub data: Vec<u8>,
}

/// Configuration change event
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    pub config_id: u64,
    pub namespace: ConfigNamespace,
    pub name: String,
    pub version_id: u64,
    pub change_type: ConfigChangeType,
}

/// Type of configuration change
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigChangeType {
    Created,
    Updated,
    Deleted,
    ReleaseUpdated,
}
