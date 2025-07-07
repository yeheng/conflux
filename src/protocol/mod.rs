use crate::app::CoreAppHandle;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod http;

/// 协议插件的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    /// 监听地址
    pub listen_addr: String,
    /// 协议特定的配置项
    pub options: HashMap<String, String>,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".to_string(),
            options: HashMap::new(),
        }
    }
}

/// 协议插件 trait
/// 
/// 所有协议插件（HTTP、gRPC 等）都必须实现这个 trait
/// 它定义了插件的生命周期和与系统核心的交互方式
#[async_trait]
pub trait ProtocolPlugin: Send + Sync {
    /// 返回协议的唯一名称
    /// 
    /// 例如: "http-rest", "grpc", "websocket"
    fn name(&self) -> &'static str;
    
    /// 启动协议服务
    /// 
    /// 这是一个长时运行的异步任务，它接收一个到应用核心的句柄，
    /// 用于执行业务操作
    /// 
    /// # Arguments
    /// * `core_handle` - 包含了 RaftClient, Store 等核心服务的句柄
    /// * `config` - 此协议实例的配置
    async fn start(&self, core_handle: CoreAppHandle, config: ProtocolConfig) -> anyhow::Result<()>;
    
    /// 获取协议的健康状态
    /// 
    /// 返回协议是否正常运行
    async fn health_check(&self) -> bool {
        // 默认实现总是返回健康状态
        true
    }
    
    /// 优雅关闭协议服务
    /// 
    /// 在应用关闭时调用，允许协议插件进行清理工作
    async fn shutdown(&self) -> anyhow::Result<()> {
        // 默认实现不做任何事情
        Ok(())
    }
}

/// 协议插件管理器
/// 
/// 负责管理和启动所有已注册的协议插件
pub struct ProtocolManager {
    plugins: Vec<Box<dyn ProtocolPlugin>>,
    configs: HashMap<String, ProtocolConfig>,
}

impl ProtocolManager {
    /// 创建新的协议管理器
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            configs: HashMap::new(),
        }
    }
    
    /// 注册协议插件
    pub fn register_plugin(&mut self, plugin: Box<dyn ProtocolPlugin>) {
        self.plugins.push(plugin);
    }
    
    /// 设置协议配置
    pub fn set_config(&mut self, protocol_name: String, config: ProtocolConfig) {
        self.configs.insert(protocol_name, config);
    }
    
    /// 启动所有已注册的协议插件
    pub async fn start_all(&self, core_handle: CoreAppHandle) -> anyhow::Result<Vec<tokio::task::JoinHandle<()>>> {
        let mut handles = Vec::new();
        
        for plugin in &self.plugins {
            let plugin_name = plugin.name();
            let config = self.configs.get(plugin_name)
                .cloned()
                .unwrap_or_default();
            
            let core_handle_clone = core_handle.clone();
            let plugin_name_owned = plugin_name.to_string();
            
            // 为每个插件创建一个独立的任务
            let handle = tokio::spawn(async move {
                // 注意：这里我们无法直接使用 plugin，因为它不是 Clone 的
                // 在实际实现中，我们需要重新设计这个部分
                tracing::info!("Starting protocol plugin: {}", plugin_name_owned);
                
                // TODO: 实际启动插件的逻辑需要在具体的插件实现中处理
                // 这里只是一个占位符
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            });
            
            handles.push(handle);
        }
        
        Ok(handles)
    }
    
    /// 获取已注册的插件数量
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
    
    /// 获取所有插件的名称
    pub fn plugin_names(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name()).collect()
    }
}

impl Default for ProtocolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::{RaftClient, Store};
    use std::sync::Arc;
    use tempfile::TempDir;

    // 测试用的协议插件实现
    struct TestProtocol {
        name: &'static str,
    }

    #[async_trait]
    impl ProtocolPlugin for TestProtocol {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn start(&self, _core_handle: CoreAppHandle, _config: ProtocolConfig) -> anyhow::Result<()> {
            // 测试实现，不做任何事情
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_protocol_config_default() {
        let config = ProtocolConfig::default();
        assert_eq!(config.listen_addr, "127.0.0.1:8080");
        assert!(config.options.is_empty());
    }

    #[tokio::test]
    async fn test_protocol_manager() {
        let mut manager = ProtocolManager::new();
        
        // 注册测试插件
        let plugin = Box::new(TestProtocol { name: "test-http" });
        manager.register_plugin(plugin);
        
        // 验证插件注册
        assert_eq!(manager.plugin_count(), 1);
        assert_eq!(manager.plugin_names(), vec!["test-http"]);
        
        // 设置配置
        let config = ProtocolConfig {
            listen_addr: "0.0.0.0:9090".to_string(),
            options: HashMap::new(),
        };
        manager.set_config("test-http".to_string(), config);
    }

    // TODO: 修复这个测试以包含AuthzService
    // #[tokio::test]
    // async fn test_core_app_handle_integration() {
    //     let temp_dir = TempDir::new().unwrap();
    //     let store = Arc::new(Store::new(temp_dir.path()).await.unwrap());
    //     let raft_client = Arc::new(RaftClient::new(store.clone()));
    //     // 需要AuthzService参数
    //     // let core_handle = CoreAppHandle::new(raft_client, store, authz_service);
    //
    //     let plugin = TestProtocol { name: "test" };
    //     let config = ProtocolConfig::default();
    //
    //     // 测试插件启动
    //     // let result = plugin.start(core_handle, config).await;
    //     // assert!(result.is_ok());
    // }
}
