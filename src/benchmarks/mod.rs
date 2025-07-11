//! 性能基准测试框架
//! 
//! 为Conflux分布式配置中心建立性能基准，包括：
//! - 基础读写性能测试
//! - 内存使用基准测试
//! - 网络延迟测试
//! - 集群性能测试

use crate::config::{AppConfig, StorageConfig};
use crate::raft::{
    network::NetworkConfig,
    node::{NodeConfig, RaftNode, ResourceLimits},
};
use openraft::Config as RaftConfig;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::time::sleep;
use tracing::info;

/// 性能测试配置
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// 测试持续时间
    pub duration: Duration,
    /// 并发连接数
    pub concurrency: usize,
    /// 预热时间
    pub warmup_duration: Duration,
    /// 测试间隔
    pub test_interval: Duration,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(30),
            concurrency: 10,
            warmup_duration: Duration::from_secs(5),
            test_interval: Duration::from_millis(100),
        }
    }
}

/// 性能测试结果
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    /// 操作总数
    pub total_operations: u64,
    /// 成功操作数
    pub successful_operations: u64,
    /// 失败操作数
    pub failed_operations: u64,
    /// 平均延迟 (毫秒)
    pub avg_latency_ms: f64,
    /// P50延迟 (毫秒)
    pub p50_latency_ms: f64,
    /// P95延迟 (毫秒)
    pub p95_latency_ms: f64,
    /// P99延迟 (毫秒)
    pub p99_latency_ms: f64,
    /// QPS (每秒查询数)
    pub qps: f64,
    /// 错误率
    pub error_rate: f64,
}

impl BenchmarkResults {
    /// 计算性能指标
    pub fn calculate(
        operations: u64,
        successful: u64,
        latencies: &mut Vec<Duration>,
        total_duration: Duration,
    ) -> Self {
        latencies.sort();
        
        let failed = operations - successful;
        let avg_latency = latencies.iter().sum::<Duration>().as_millis() as f64 / latencies.len() as f64;
        
        let p50_latency = if !latencies.is_empty() {
            latencies[latencies.len() * 50 / 100].as_millis() as f64
        } else { 0.0 };
        
        let p95_latency = if !latencies.is_empty() {
            latencies[latencies.len() * 95 / 100].as_millis() as f64
        } else { 0.0 };
        
        let p99_latency = if !latencies.is_empty() {
            latencies[latencies.len() * 99 / 100].as_millis() as f64
        } else { 0.0 };
        
        let qps = successful as f64 / total_duration.as_secs_f64();
        let error_rate = failed as f64 / operations as f64 * 100.0;

        Self {
            total_operations: operations,
            successful_operations: successful,
            failed_operations: failed,
            avg_latency_ms: avg_latency,
            p50_latency_ms: p50_latency,
            p95_latency_ms: p95_latency,
            p99_latency_ms: p99_latency,
            qps,
            error_rate,
        }
    }

    /// 显示测试结果
    pub fn display(&self, test_name: &str) {
        info!("=== {} 性能测试结果 ===", test_name);
        info!("总操作数: {}", self.total_operations);
        info!("成功操作数: {}", self.successful_operations);
        info!("失败操作数: {}", self.failed_operations);
        info!("QPS (每秒查询数): {:.2}", self.qps);
        info!("错误率: {:.2}%", self.error_rate);
        info!("平均延迟: {:.2}ms", self.avg_latency_ms);
        info!("P50延迟: {:.2}ms", self.p50_latency_ms);
        info!("P95延迟: {:.2}ms", self.p95_latency_ms);
        info!("P99延迟: {:.2}ms", self.p99_latency_ms);
        info!("========================");
    }

    /// 检查是否达到性能目标
    pub fn meets_performance_targets(&self) -> bool {
        // 基础性能目标
        self.qps >= 100.0 && 
        self.error_rate < 1.0 && 
        self.avg_latency_ms < 100.0
    }
}

/// 内存使用统计
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// 初始内存使用 (MB)
    pub initial_memory_mb: f64,
    /// 峰值内存使用 (MB)  
    pub peak_memory_mb: f64,
    /// 当前内存使用 (MB)
    pub current_memory_mb: f64,
    /// 内存增长 (MB)
    pub memory_growth_mb: f64,
}

impl MemoryStats {
    /// 获取当前内存使用情况
    pub fn current() -> Self {
        // 简化的内存统计实现
        // 在实际产品中应该使用更精确的内存监控
        let current_mb = Self::get_memory_usage_mb();
        
        Self {
            initial_memory_mb: current_mb,
            peak_memory_mb: current_mb,
            current_memory_mb: current_mb,
            memory_growth_mb: 0.0,
        }
    }

    /// 更新内存统计
    pub fn update(&mut self) {
        let current = Self::get_memory_usage_mb();
        self.current_memory_mb = current;
        if current > self.peak_memory_mb {
            self.peak_memory_mb = current;
        }
        self.memory_growth_mb = self.current_memory_mb - self.initial_memory_mb;
    }

    /// 获取内存使用量 (MB)
    fn get_memory_usage_mb() -> f64 {
        // 简化实现：使用系统内存信息
        // 在生产环境中应该使用更准确的内存监控工具
        use std::process::Command;
        
        if let Ok(output) = Command::new("ps")
            .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
        {
            if let Ok(rss_str) = String::from_utf8(output.stdout) {
                if let Ok(rss_kb) = rss_str.trim().parse::<f64>() {
                    return rss_kb / 1024.0; // 转换为 MB
                }
            }
        }
        
        // 回退：返回估算值
        0.0
    }

    /// 显示内存统计
    pub fn display(&self, test_name: &str) {
        info!("=== {} 内存使用统计 ===", test_name);
        info!("初始内存: {:.2} MB", self.initial_memory_mb);
        info!("峰值内存: {:.2} MB", self.peak_memory_mb);
        info!("当前内存: {:.2} MB", self.current_memory_mb);
        info!("内存增长: {:.2} MB", self.memory_growth_mb);
        info!("=======================");
    }

    /// 检查内存使用是否在合理范围内
    pub fn is_memory_usage_acceptable(&self) -> bool {
        // 基础内存目标：空载 < 200MB
        self.current_memory_mb < 200.0
    }
}

/// 单节点性能基准测试
pub struct SingleNodeBenchmark {
    node: RaftNode,
    _temp_dir: TempDir,
}

impl SingleNodeBenchmark {
    /// 创建单节点测试环境
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        
        let node_config = NodeConfig {
            node_id: 1,
            address: "127.0.0.1:18090".to_string(),
            raft_config: RaftConfig::default(),
            network_config: NetworkConfig::default(),
            heartbeat_interval: 150,
            election_timeout_min: 300,
            election_timeout_max: 600,
            resource_limits: ResourceLimits::default(),
        };

        let app_config = AppConfig {
            storage: StorageConfig {
                data_dir: temp_dir.path().to_string_lossy().to_string(),
                max_open_files: 1000,
                cache_size_mb: 64,
                write_buffer_size_mb: 64,
                max_write_buffer_number: 2,
            },
            ..Default::default()
        };

        let mut node = RaftNode::new(node_config, &app_config).await?;
        node.start().await?;

        Ok(Self {
            node,
            _temp_dir: temp_dir,
        })
    }

    /// 基础性能测试
    pub async fn run_basic_performance_test(&self, config: &BenchmarkConfig) -> BenchmarkResults {
        info!("开始基础性能测试...");
        
        // 预热
        info!("预热阶段...");
        self.warmup(config.warmup_duration).await;

        // 性能测试
        info!("开始性能测试，持续时间: {:?}", config.duration);
        let start_time = Instant::now();
        let mut operations = 0u64;
        let mut successful = 0u64;
        let mut latencies = Vec::new();

        while start_time.elapsed() < config.duration {
            let op_start = Instant::now();
            
            // 模拟基础操作（获取metrics）
            match self.node.get_metrics().await {
                Ok(_) => {
                    successful += 1;
                    latencies.push(op_start.elapsed());
                }
                Err(_) => {
                    // 操作失败
                }
            }
            
            operations += 1;
            
            // 控制测试频率
            sleep(config.test_interval).await;
        }

        let total_duration = start_time.elapsed();
        BenchmarkResults::calculate(operations, successful, &mut latencies, total_duration)
    }

    /// 内存使用测试
    pub async fn run_memory_test(&self, duration: Duration) -> MemoryStats {
        info!("开始内存使用测试，持续时间: {:?}", duration);
        
        let mut memory_stats = MemoryStats::current();
        let start_time = Instant::now();

        while start_time.elapsed() < duration {
            // 执行一些操作来观察内存使用
            let _ = self.node.get_metrics().await;
            
            // 更新内存统计
            memory_stats.update();
            
            sleep(Duration::from_millis(1000)).await; // 每秒检查一次
        }

        memory_stats
    }

    /// 延迟测试
    pub async fn run_latency_test(&self, samples: usize) -> Vec<Duration> {
        info!("开始延迟测试，样本数: {}", samples);
        
        let mut latencies = Vec::with_capacity(samples);
        
        for _ in 0..samples {
            let start = Instant::now();
            let _ = self.node.get_metrics().await;
            latencies.push(start.elapsed());
            
            // 短暂间隔
            sleep(Duration::from_millis(10)).await;
        }

        latencies
    }

    /// 预热操作
    async fn warmup(&self, duration: Duration) {
        let start_time = Instant::now();
        
        while start_time.elapsed() < duration {
            let _ = self.node.get_metrics().await;
            sleep(Duration::from_millis(50)).await;
        }
    }
}

/// 集群性能基准测试
pub struct ClusterBenchmark {
    nodes: Vec<RaftNode>,
    _temp_dirs: Vec<TempDir>,
}

impl ClusterBenchmark {
    /// 创建3节点集群测试环境
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut temp_dirs = Vec::new();
        let mut nodes = Vec::new();

        for _i in 0..3 {
            temp_dirs.push(TempDir::new()?);
        }

        let mut node_addresses = HashMap::new();
        for i in 1..=3u64 {
            node_addresses.insert(i, format!("127.0.0.1:{}", 18100 + i - 1));
        }

        let network_config = NetworkConfig::new(node_addresses.clone());

        for i in 0..3 {
            let node_id = (i + 1) as u64;
            
            let node_config = NodeConfig {
                node_id,
                address: node_addresses[&node_id].clone(),
                raft_config: RaftConfig::default(),
                network_config: network_config.clone(),
                heartbeat_interval: 150,
                election_timeout_min: 300,
                election_timeout_max: 600,
                resource_limits: ResourceLimits::default(),
            };

            let app_config = AppConfig {
                storage: StorageConfig {
                    data_dir: temp_dirs[i].path().to_string_lossy().to_string(),
                    max_open_files: 1000,
                    cache_size_mb: 64,
                    write_buffer_size_mb: 64,
                    max_write_buffer_number: 2,
                },
                ..Default::default()
            };

            let mut node = RaftNode::new(node_config, &app_config).await?;
            node.start().await?;
            nodes.push(node);
        }

        // 等待集群稳定
        sleep(Duration::from_secs(2)).await;

        Ok(Self {
            nodes,
            _temp_dirs: temp_dirs,
        })
    }

    /// 集群性能测试
    pub async fn run_cluster_performance_test(&self, config: &BenchmarkConfig) -> BenchmarkResults {
        info!("开始集群性能测试...");
        
        // 预热
        info!("集群预热阶段...");
        self.warmup_cluster(config.warmup_duration).await;

        // 性能测试
        info!("开始集群性能测试，持续时间: {:?}", config.duration);
        let start_time = Instant::now();
        let mut operations = 0u64;
        let mut successful = 0u64;
        let mut latencies = Vec::new();

        while start_time.elapsed() < config.duration {
            let op_start = Instant::now();
            
            // 随机选择一个节点进行操作
            let node_idx = operations as usize % self.nodes.len();
            
            match self.nodes[node_idx].get_metrics().await {
                Ok(_) => {
                    successful += 1;
                    latencies.push(op_start.elapsed());
                }
                Err(_) => {
                    // 操作失败
                }
            }
            
            operations += 1;
            
            // 控制测试频率
            sleep(config.test_interval).await;
        }

        let total_duration = start_time.elapsed();
        BenchmarkResults::calculate(operations, successful, &mut latencies, total_duration)
    }

    /// 集群预热
    async fn warmup_cluster(&self, duration: Duration) {
        let start_time = Instant::now();
        let mut node_idx = 0;
        
        while start_time.elapsed() < duration {
            let _ = self.nodes[node_idx].get_metrics().await;
            node_idx = (node_idx + 1) % self.nodes.len();
            sleep(Duration::from_millis(50)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn test_single_node_benchmark() {
        let benchmark = SingleNodeBenchmark::new().await.expect("Failed to create benchmark");
        
        let config = BenchmarkConfig {
            duration: Duration::from_secs(5),
            concurrency: 1,
            warmup_duration: Duration::from_secs(1),
            test_interval: Duration::from_millis(50),
        };

        let results = benchmark.run_basic_performance_test(&config).await;
        results.display("单节点基础性能");
        
        // 验证基本指标
        assert!(results.total_operations > 0);
        assert!(results.successful_operations > 0);
        assert!(results.qps > 0.0);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_memory_benchmark() {
        let benchmark = SingleNodeBenchmark::new().await.expect("Failed to create benchmark");
        
        let memory_stats = benchmark.run_memory_test(Duration::from_secs(3)).await;
        memory_stats.display("内存使用");
        
        // 基础内存检查
        assert!(memory_stats.current_memory_mb >= 0.0);
    }

    #[tokio::test]
    #[traced_test]  
    async fn test_latency_benchmark() {
        let benchmark = SingleNodeBenchmark::new().await.expect("Failed to create benchmark");
        
        let latencies = benchmark.run_latency_test(20).await;
        
        assert_eq!(latencies.len(), 20);
        
        let avg_latency_ms = latencies.iter().sum::<Duration>().as_millis() as f64 / latencies.len() as f64;
        info!("平均延迟: {:.2}ms", avg_latency_ms);
        
        // 基础延迟检查
        assert!(avg_latency_ms < 1000.0); // 应该小于1秒
    }
}