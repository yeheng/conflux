use crate::auth::AuthzService;
use crate::raft::{RaftClient, Store};
use std::sync::Arc;

/// 核心应用句柄，封装了所有核心服务的引用
/// 这个结构体是协议层与核心业务逻辑之间的桥梁
#[derive(Clone)]
pub struct CoreAppHandle {
    /// Raft 客户端，用于处理分布式共识操作
    pub raft_client: Arc<RaftClient>,
    
    /// 存储实例，用于直接访问数据（读操作）
    pub store: Arc<Store>,

    /// 认证授权服务
    pub authz_service: Arc<AuthzService>,

    // TODO: 在后续的 Epic 中添加更多服务
    // pub metadata_service: Arc<MetadataService>,
    // pub watch_service: Arc<WatchService>,
}

impl CoreAppHandle {
    /// 创建新的核心应用句柄
    pub fn new(raft_client: Arc<RaftClient>, store: Arc<Store>, authz_service: Arc<AuthzService>) -> Self {
        Self {
            raft_client,
            store,
            authz_service,
        }
    }
    
    /// 获取 Raft 客户端的引用
    pub fn raft_client(&self) -> &RaftClient {
        &self.raft_client
    }
    
    /// 获取存储实例的引用
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// 获取认证授权服务的引用
    pub fn authz_service(&self) -> &AuthzService {
        &self.authz_service
    }
}

// TODO: 更新测试以包含AuthzService
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::raft::Store;
//     use tempfile::TempDir;

//     #[tokio::test]
//     async fn test_core_app_handle_creation() {
//         let temp_dir = TempDir::new().unwrap();
//         let store = Arc::new(Store::new(temp_dir.path()).await.unwrap());
//         let raft_client = Arc::new(RaftClient::new(store.clone()));

//         let handle = CoreAppHandle::new(raft_client.clone(), store.clone());

//         // 验证句柄正确包含了服务引用
//         assert!(Arc::ptr_eq(&handle.raft_client, &raft_client));
//         assert!(Arc::ptr_eq(&handle.store, &store));
//     }
// }
