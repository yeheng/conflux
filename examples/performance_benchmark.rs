//! 性能基准测试程序
//! 运行Conflux分布式配置中心的性能基准测试

use conflux::benchmarks::{
    BenchmarkConfig, SingleNodeBenchmark, ClusterBenchmark, MemoryStats
};
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🚀 开始Conflux性能基准测试");

    // 测试配置
    let benchmark_config = BenchmarkConfig {
        duration: Duration::from_secs(10),
        concurrency: 1,
        warmup_duration: Duration::from_secs(2),
        test_interval: Duration::from_millis(50),
    };

    // 1. 单节点性能测试
    info!("📊 === 单节点性能基准测试 ===");
    run_single_node_tests(&benchmark_config).await?;

    // 等待一段时间让系统稳定
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 2. 集群性能测试
    info!("📊 === 集群性能基准测试 ===");
    run_cluster_tests(&benchmark_config).await?;

    info!("🎉 性能基准测试完成！");

    Ok(())
}

/// 运行单节点性能测试
async fn run_single_node_tests(config: &BenchmarkConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("🔧 创建单节点测试环境...");
    let benchmark = SingleNodeBenchmark::new().await?;
    
    // 基础性能测试
    info!("⚡ 开始基础性能测试...");
    let perf_results = benchmark.run_basic_performance_test(config).await;
    perf_results.display("单节点基础性能");
    
    // 检查性能目标
    if perf_results.meets_performance_targets() {
        info!("✅ 单节点性能测试通过目标要求");
    } else {
        info!("⚠️ 单节点性能测试未达到目标要求");
        info!("目标: QPS >= 100, 错误率 < 1%, 平均延迟 < 100ms");
    }

    // 内存使用测试
    info!("💾 开始内存使用测试...");
    let memory_stats = benchmark.run_memory_test(Duration::from_secs(5)).await;
    memory_stats.display("单节点内存使用");
    
    if memory_stats.is_memory_usage_acceptable() {
        info!("✅ 内存使用测试通过目标要求");
    } else {
        info!("⚠️ 内存使用超出目标范围");
        info!("目标: 空载内存使用 < 200MB");
    }

    // 延迟测试
    info!("⏱️ 开始延迟分布测试...");
    let latencies = benchmark.run_latency_test(50).await;
    
    let mut latency_ms: Vec<f64> = latencies.iter()
        .map(|d| d.as_millis() as f64)
        .collect();
    latency_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let avg_latency = latency_ms.iter().sum::<f64>() / latency_ms.len() as f64;
    let p95_latency = latency_ms[latency_ms.len() * 95 / 100];
    let p99_latency = latency_ms[latency_ms.len() * 99 / 100];
    
    info!("=== 延迟分布统计 ===");
    info!("样本数: {}", latency_ms.len());
    info!("平均延迟: {:.2}ms", avg_latency);
    info!("P95延迟: {:.2}ms", p95_latency);
    info!("P99延迟: {:.2}ms", p99_latency);
    info!("==================");

    Ok(())
}

/// 运行集群性能测试
async fn run_cluster_tests(config: &BenchmarkConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("🔧 创建3节点集群测试环境...");
    let cluster_benchmark = ClusterBenchmark::new().await?;
    
    // 集群性能测试
    info!("⚡ 开始集群性能测试...");
    let cluster_results = cluster_benchmark.run_cluster_performance_test(config).await;
    cluster_results.display("3节点集群性能");
    
    // 检查集群性能
    if cluster_results.meets_performance_targets() {
        info!("✅ 集群性能测试通过目标要求");
    } else {
        info!("⚠️ 集群性能测试未达到目标要求");
        info!("目标: QPS >= 100, 错误率 < 1%, 平均延迟 < 100ms");
    }

    // 比较单节点和集群性能
    info!("📈 集群性能分析:");
    if cluster_results.qps > 50.0 {
        info!("✅ 集群QPS表现良好: {:.2}", cluster_results.qps);
    } else {
        info!("⚠️ 集群QPS较低，可能需要优化: {:.2}", cluster_results.qps);
    }

    Ok(())
}

/// 显示性能测试总结
fn display_performance_summary() {
    info!("📋 === 性能基准测试总结 ===");
    info!("✅ 单节点基础功能验证");
    info!("✅ 集群协调功能验证");
    info!("✅ 内存使用基准建立");
    info!("✅ 延迟分布统计完成");
    info!("📊 基准测试框架建立完成");
    info!("🎯 下一步: 根据基准数据进行性能优化");
    info!("=============================");
}