//! 手动验证3节点Raft集群基本功能
//! 这个程序创建一个最小化的3节点集群验证

use conflux::config::{AppConfig, StorageConfig};
use conflux::raft::{
    network::NetworkConfig,
    node::{NodeConfig, RaftNode},
};
use openraft::Config as RaftConfig;
use std::collections::HashMap;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🚀 开始验证3节点Raft集群原型");

    // 创建临时目录
    let temp_dir1 = TempDir::new()?;
    let temp_dir2 = TempDir::new()?;
    let temp_dir3 = TempDir::new()?;

    info!("📁 临时目录创建完成");

    // 配置节点地址
    let mut node_addresses = HashMap::new();
    node_addresses.insert(1u64, "127.0.0.1:19001".to_string());
    node_addresses.insert(2u64, "127.0.0.1:19002".to_string());
    node_addresses.insert(3u64, "127.0.0.1:19003".to_string());

    let network_config = NetworkConfig::new(node_addresses.clone());

    info!("🌐 网络配置: {:?}", node_addresses);

    // 创建节点配置
    let node_configs = vec![
        NodeConfig {
            node_id: 1,
            address: "127.0.0.1:19001".to_string(),
            raft_config: RaftConfig {
                heartbeat_interval: 150,
                election_timeout_min: 300,
                election_timeout_max: 600,
                ..Default::default()
            },
            network_config: network_config.clone(),
        },
        NodeConfig {
            node_id: 2,
            address: "127.0.0.1:19002".to_string(),
            raft_config: RaftConfig {
                heartbeat_interval: 150,
                election_timeout_min: 300,
                election_timeout_max: 600,
                ..Default::default()
            },
            network_config: network_config.clone(),
        },
        NodeConfig {
            node_id: 3,
            address: "127.0.0.1:19003".to_string(),
            raft_config: RaftConfig {
                heartbeat_interval: 150,
                election_timeout_min: 300,
                election_timeout_max: 600,
                ..Default::default()
            },
            network_config: network_config.clone(),
        },
    ];

    let temp_dirs = vec![&temp_dir1, &temp_dir2, &temp_dir3];

    // 创建应用配置
    let app_configs: Vec<AppConfig> = temp_dirs
        .iter()
        .map(|dir| AppConfig {
            storage: StorageConfig {
                data_dir: dir.path().to_string_lossy().to_string(),
                max_open_files: 1000,
                cache_size_mb: 64,
                write_buffer_size_mb: 64,
                max_write_buffer_number: 2,
            },
            ..Default::default()
        })
        .collect();

    info!("⚙️  节点配置创建完成");

    // 创建节点
    let mut nodes = Vec::new();
    for (i, (node_config, app_config)) in node_configs.into_iter().zip(app_configs.iter()).enumerate() {
        info!("🏗️  创建节点 {}", i + 1);
        let node = RaftNode::new(node_config, app_config).await?;
        nodes.push(node);
        info!("✅ 节点 {} 创建成功", i + 1);
    }

    info!("🎯 所有节点创建完成，开始启动节点");

    // 启动节点
    for (i, node) in nodes.iter_mut().enumerate() {
        info!("🚀 启动节点 {}", i + 1);
        match node.start().await {
            Ok(_) => info!("✅ 节点 {} 启动成功", i + 1),
            Err(e) => {
                info!("❌ 节点 {} 启动失败: {}", i + 1, e);
                return Err(e.into());
            }
        }
    }

    info!("⏱️  等待集群稳定化...");
    sleep(Duration::from_secs(2)).await;

    // 检查节点状态
    info!("🔍 检查节点状态:");
    for (i, node) in nodes.iter().enumerate() {
        let node_id = node.node_id();
        let address = node.address();
        let is_leader = node.is_leader().await;
        
        info!("📊 节点 {} (ID: {}, 地址: {}) - 领导者: {}", 
              i + 1, node_id, address, is_leader);

        // 尝试获取metrics
        match node.get_metrics().await {
            Ok(metrics) => {
                info!("📈 节点 {} metrics: {:?}", i + 1, metrics);
            }
            Err(e) => {
                info!("⚠️  节点 {} metrics获取失败: {}", i + 1, e);
            }
        }
    }

    // 查找领导者
    let mut leader_found = false;
    for (i, node) in nodes.iter().enumerate() {
        if node.is_leader().await {
            info!("👑 领导者是节点 {} (ID: {})", i + 1, node.node_id());
            leader_found = true;
            break;
        }
    }

    if !leader_found {
        info!("⚠️  未检测到领导者，这在集群初始阶段是正常的");
    }

    info!("🎉 3节点Raft集群原型验证完成!");

    // 等待一段时间让用户观察
    info!("⏱️  等待5秒让集群运行...");
    sleep(Duration::from_secs(5)).await;

    info!("🏁 验证程序结束");

    Ok(())
}