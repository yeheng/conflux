//! 资源限制器模块
//!
//! 提供客户端请求的资源限制和速率控制功能

use super::config::ResourceLimits;
use crate::error::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tracing::warn;

/// 客户端资源限制器
/// 
/// 用于管理客户端请求的资源限制，包括并发数、内存使用量和速率限制
/// 
/// # Examples
/// 
/// ```rust
/// use conflux::raft::node::{ResourceLimiter, ResourceLimits};
/// 
/// let limits = ResourceLimits::default();
/// let limiter = ResourceLimiter::new(limits);
/// ```
#[derive(Debug)]
pub struct ResourceLimiter {
    /// 资源限制配置
    limits: ResourceLimits,
    /// 并发请求限制信号量
    concurrent_requests: Semaphore,
    /// 当前内存使用量（待处理请求）
    current_memory_usage: Arc<AtomicUsize>,
    /// 每个客户端的速率限制状态
    rate_limit_state: RwLock<HashMap<String, RateLimitState>>,
    /// 全局请求计数（用于指标）
    total_requests: AtomicU32,
    /// 被拒绝的请求计数
    rejected_requests: AtomicU32,
}

/// 客户端速率限制状态
#[derive(Debug, Clone)]
struct RateLimitState {
    /// 当前时间窗口内的请求计数
    request_count: u32,
    /// 时间窗口开始时间
    window_start: Instant,
}

impl ResourceLimiter {
    /// 创建新的资源限制器
    /// 
    /// # Arguments
    /// 
    /// * `limits` - 资源限制配置
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::node::{ResourceLimiter, ResourceLimits};
    /// 
    /// let limits = ResourceLimits::default();
    /// let limiter = ResourceLimiter::new(limits);
    /// ```
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            concurrent_requests: Semaphore::new(limits.max_concurrent_requests as usize),
            limits,
            current_memory_usage: Arc::new(AtomicUsize::new(0)),
            rate_limit_state: RwLock::new(HashMap::new()),
            total_requests: AtomicU32::new(0),
            rejected_requests: AtomicU32::new(0),
        }
    }

    /// 检查请求是否被允许处理
    /// 
    /// 检查请求大小、内存使用量、速率限制和并发数限制
    /// 
    /// # Arguments
    /// 
    /// * `request_size` - 请求大小（字节）
    /// * `client_id` - 可选的客户端ID，用于速率限制
    /// 
    /// # Returns
    /// 
    /// 如果请求被允许，返回RequestPermit；否则返回错误
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::node::{ResourceLimiter, ResourceLimits};
    /// 
    /// # tokio_test::block_on(async {
    /// let limits = ResourceLimits::default();
    /// let limiter = ResourceLimiter::new(limits);
    /// 
    /// let permit = limiter.check_request_allowed(1024, Some("client1")).await;
    /// assert!(permit.is_ok());
    /// # });
    /// ```
    pub async fn check_request_allowed(&self, request_size: usize, client_id: Option<&str>) -> Result<RequestPermit<'_>> {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        // 检查请求大小限制
        if request_size > self.limits.max_request_size {
            self.rejected_requests.fetch_add(1, Ordering::Relaxed);
            return Err(crate::error::ConfluxError::raft(format!(
                "Request size {} exceeds limit {}",
                request_size, self.limits.max_request_size
            )));
        }

        // 检查内存使用量限制
        let current_memory = self.current_memory_usage.load(Ordering::Relaxed);
        if current_memory + request_size > self.limits.max_memory_usage {
            self.rejected_requests.fetch_add(1, Ordering::Relaxed);
            return Err(crate::error::ConfluxError::raft(format!(
                "Memory usage limit exceeded: current={}, request={}, limit={}",
                current_memory, request_size, self.limits.max_memory_usage
            )));
        }

        // 检查客户端速率限制
        if let Some(client) = client_id {
            let mut state_map = self.rate_limit_state.write().await;
            let now = Instant::now();
            
            let client_state = state_map.entry(client.to_string()).or_insert_with(|| RateLimitState {
                request_count: 0,
                window_start: now,
            });

            // 如果超过1秒则重置时间窗口
            if now.duration_since(client_state.window_start) >= Duration::from_secs(1) {
                client_state.request_count = 0;
                client_state.window_start = now;
            }

            // 检查速率限制
            if client_state.request_count >= self.limits.max_requests_per_second {
                self.rejected_requests.fetch_add(1, Ordering::Relaxed);
                return Err(crate::error::ConfluxError::raft(format!(
                    "Rate limit exceeded for client {}: {} requests/second",
                    client, client_state.request_count
                )));
            }

            client_state.request_count += 1;
        }

        // 尝试获取并发请求许可
        match self.concurrent_requests.try_acquire() {
            Ok(permit) => {
                // 为此请求预留内存
                self.current_memory_usage.fetch_add(request_size, Ordering::Relaxed);
                
                Ok(RequestPermit {
                    _permit: permit,
                    request_size,
                    memory_tracker: self.current_memory_usage.clone(),
                })
            }
            Err(_) => {
                self.rejected_requests.fetch_add(1, Ordering::Relaxed);
                Err(crate::error::ConfluxError::raft(format!(
                    "Too many concurrent requests: limit={}",
                    self.limits.max_concurrent_requests
                )))
            }
        }
    }

    /// 获取资源使用统计信息
    /// 
    /// # Returns
    /// 
    /// 返回当前的资源使用统计
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::node::{ResourceLimiter, ResourceLimits};
    /// 
    /// let limits = ResourceLimits::default();
    /// let limiter = ResourceLimiter::new(limits);
    /// let stats = limiter.get_resource_stats();
    /// 
    /// println!("Total requests: {}", stats.total_requests);
    /// ```
    pub fn get_resource_stats(&self) -> ResourceStats {
        ResourceStats {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            rejected_requests: self.rejected_requests.load(Ordering::Relaxed),
            current_memory_usage: self.current_memory_usage.load(Ordering::Relaxed),
            available_permits: self.concurrent_requests.available_permits(),
            max_concurrent_requests: self.limits.max_concurrent_requests as usize,
        }
    }

    /// 更新资源限制配置
    /// 
    /// # Arguments
    /// 
    /// * `new_limits` - 新的资源限制配置
    /// 
    /// # Note
    /// 
    /// 某些更改（如并发数限制）可能需要重启才能完全生效
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use conflux::raft::node::{ResourceLimiter, ResourceLimits};
    /// 
    /// let limits = ResourceLimits::default();
    /// let mut limiter = ResourceLimiter::new(limits);
    /// 
    /// let new_limits = ResourceLimits::new(200, 100, 2_000_000, 100_000_000, 10000);
    /// limiter.update_limits(new_limits);
    /// ```
    pub fn update_limits(&mut self, new_limits: ResourceLimits) {
        self.limits = new_limits;
        // 注意：运行时更改信号量许可数是复杂的
        // 这是一个简化的实现
        warn!("Resource limits updated - some changes may require restart");
    }

    /// 获取当前资源限制配置
    /// 
    /// # Returns
    /// 
    /// 返回当前的资源限制配置
    pub fn get_limits(&self) -> &ResourceLimits {
        &self.limits
    }
}

/// 请求许可的RAII守卫
/// 
/// 当守卫被丢弃时，会自动释放相关资源（内存和并发许可）
pub struct RequestPermit<'a> {
    _permit: tokio::sync::SemaphorePermit<'a>,
    request_size: usize,
    memory_tracker: Arc<AtomicUsize>,
}

impl Drop for RequestPermit<'_> {
    fn drop(&mut self) {
        // 请求完成时释放内存
        self.memory_tracker.fetch_sub(self.request_size, Ordering::Relaxed);
    }
}

/// 资源使用统计信息
/// 
/// 提供当前资源使用情况的快照
#[derive(Debug, Clone)]
pub struct ResourceStats {
    /// 总请求数
    pub total_requests: u32,
    /// 被拒绝的请求数
    pub rejected_requests: u32,
    /// 当前内存使用量（字节）
    pub current_memory_usage: usize,
    /// 可用的并发许可数
    pub available_permits: usize,
    /// 最大并发请求数
    pub max_concurrent_requests: usize,
}

impl ResourceStats {
    /// 计算请求成功率
    /// 
    /// # Returns
    /// 
    /// 返回0.0到1.0之间的成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            let successful_requests = self.total_requests - self.rejected_requests;
            successful_requests as f64 / self.total_requests as f64
        }
    }

    /// 计算内存使用率
    /// 
    /// # Arguments
    /// 
    /// * `max_memory` - 最大内存限制
    /// 
    /// # Returns
    /// 
    /// 返回0.0到1.0之间的内存使用率
    pub fn memory_usage_rate(&self, max_memory: usize) -> f64 {
        if max_memory == 0 {
            0.0
        } else {
            self.current_memory_usage as f64 / max_memory as f64
        }
    }

    /// 计算并发使用率
    /// 
    /// # Returns
    /// 
    /// 返回0.0到1.0之间的并发使用率
    pub fn concurrency_usage_rate(&self) -> f64 {
        if self.max_concurrent_requests == 0 {
            0.0
        } else {
            let used_permits = self.max_concurrent_requests - self.available_permits;
            used_permits as f64 / self.max_concurrent_requests as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limiter_creation() {
        let limits = ResourceLimits::default();
        let limiter = ResourceLimiter::new(limits);
        
        let stats = limiter.get_resource_stats();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.rejected_requests, 0);
        assert_eq!(stats.current_memory_usage, 0);
    }

    #[tokio::test]
    async fn test_request_size_limit() {
        let limits = ResourceLimits::default();
        let limiter = ResourceLimiter::new(limits);
        
        // 请求大小超过限制
        let result = limiter.check_request_allowed(2 * 1024 * 1024, None).await;
        assert!(result.is_err());
        
        let stats = limiter.get_resource_stats();
        assert_eq!(stats.rejected_requests, 1);
    }

    #[tokio::test]
    async fn test_memory_limit() {
        let mut limits = ResourceLimits::default();
        limits.max_memory_usage = 1024; // 1KB
        limits.max_request_size = 512; // 512B

        let limiter = ResourceLimiter::new(limits);

        // 第一个请求应该成功
        let _permit1 = limiter.check_request_allowed(512, None).await.unwrap();

        // 第二个请求应该失败（内存不足）
        let result = limiter.check_request_allowed(513, None).await; // 512 + 513 > 1024
        assert!(result.is_err());

        // 释放第一个permit后，应该可以再次请求
        drop(_permit1);
        let result = limiter.check_request_allowed(512, None).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_stats() {
        let stats = ResourceStats {
            total_requests: 100,
            rejected_requests: 10,
            current_memory_usage: 1024,
            available_permits: 40,
            max_concurrent_requests: 50,
        };
        
        assert_eq!(stats.success_rate(), 0.9);
        assert_eq!(stats.memory_usage_rate(2048), 0.5);
        assert_eq!(stats.concurrency_usage_rate(), 0.2);
    }
}
