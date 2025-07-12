//! 超时配置验证模块
//!
//! 提供Raft超时配置的验证功能

use crate::error::{ConfluxError, Result};
use tracing::debug;

/// 超时验证器
///
/// 专门负责Raft超时配置的验证
pub struct TimeoutValidator;

impl TimeoutValidator {
    /// 创建新的超时验证器
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::TimeoutValidator;
    ///
    /// let validator = TimeoutValidator::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// 验证超时配置值
    ///
    /// 检查心跳间隔和选举超时的合理性及其相互关系
    ///
    /// # Arguments
    ///
    /// * `heartbeat_interval` - 可选的心跳间隔（毫秒）
    /// * `election_timeout_min` - 可选的选举超时最小值（毫秒）
    /// * `election_timeout_max` - 可选的选举超时最大值（毫秒）
    ///
    /// # Returns
    ///
    /// 如果配置合理返回Ok(())，否则返回错误
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::TimeoutValidator;
    ///
    /// let validator = TimeoutValidator::new();
    ///
    /// // 有效的超时配置
    /// assert!(validator.validate_timeout_config(Some(100), Some(300), Some(600)).is_ok());
    ///
    /// // 无效的超时配置
    /// assert!(validator.validate_timeout_config(Some(0), None, None).is_err());
    /// ```
    pub fn validate_timeout_config(
        &self,
        heartbeat_interval: Option<u64>,
        election_timeout_min: Option<u64>,
        election_timeout_max: Option<u64>,
    ) -> Result<()> {
        debug!("Validating timeout configuration");

        // 验证心跳间隔
        if let Some(heartbeat) = heartbeat_interval {
            self.validate_heartbeat_interval(heartbeat)?;
        }

        // 验证选举超时最小值
        if let Some(min_timeout) = election_timeout_min {
            self.validate_election_timeout_min(min_timeout)?;
        }

        // 验证选举超时最大值
        if let Some(max_timeout) = election_timeout_max {
            self.validate_election_timeout_max(max_timeout)?;
        }

        // 验证超时值之间的关系
        self.validate_timeout_relationships(
            heartbeat_interval,
            election_timeout_min,
            election_timeout_max,
        )?;

        debug!("Timeout configuration is valid");
        Ok(())
    }

    /// 验证心跳间隔
    ///
    /// # Arguments
    ///
    /// * `heartbeat_interval` - 心跳间隔（毫秒）
    ///
    /// # Returns
    ///
    /// 如果心跳间隔合理返回Ok(())，否则返回错误
    pub fn validate_heartbeat_interval(&self, heartbeat_interval: u64) -> Result<()> {
        debug!("Validating heartbeat interval: {}ms", heartbeat_interval);

        if heartbeat_interval == 0 {
            return Err(ConfluxError::validation(
                "Heartbeat interval cannot be zero".to_string(),
            ));
        }

        if heartbeat_interval < 10 {
            return Err(ConfluxError::validation(
                "Heartbeat interval cannot be less than 10ms".to_string(),
            ));
        }

        if heartbeat_interval > 10000 {
            // 10 seconds max
            return Err(ConfluxError::validation(
                "Heartbeat interval cannot exceed 10000ms".to_string(),
            ));
        }

        debug!("Heartbeat interval {}ms is valid", heartbeat_interval);
        Ok(())
    }

    /// 验证选举超时最小值
    ///
    /// # Arguments
    ///
    /// * `election_timeout_min` - 选举超时最小值（毫秒）
    ///
    /// # Returns
    ///
    /// 如果选举超时最小值合理返回Ok(())，否则返回错误
    pub fn validate_election_timeout_min(&self, election_timeout_min: u64) -> Result<()> {
        debug!(
            "Validating election timeout min: {}ms",
            election_timeout_min
        );

        if election_timeout_min == 0 {
            return Err(ConfluxError::validation(
                "Election timeout min cannot be zero".to_string(),
            ));
        }

        if election_timeout_min < 50 {
            return Err(ConfluxError::validation(
                "Election timeout min cannot be less than 50ms".to_string(),
            ));
        }

        if election_timeout_min > 30000 {
            // 30 seconds max
            return Err(ConfluxError::validation(
                "Election timeout min cannot exceed 30000ms".to_string(),
            ));
        }

        debug!("Election timeout min {}ms is valid", election_timeout_min);
        Ok(())
    }

    /// 验证选举超时最大值
    ///
    /// # Arguments
    ///
    /// * `election_timeout_max` - 选举超时最大值（毫秒）
    ///
    /// # Returns
    ///
    /// 如果选举超时最大值合理返回Ok(())，否则返回错误
    pub fn validate_election_timeout_max(&self, election_timeout_max: u64) -> Result<()> {
        debug!(
            "Validating election timeout max: {}ms",
            election_timeout_max
        );

        if election_timeout_max == 0 {
            return Err(ConfluxError::validation(
                "Election timeout max cannot be zero".to_string(),
            ));
        }

        if election_timeout_max < 100 {
            return Err(ConfluxError::validation(
                "Election timeout max cannot be less than 100ms".to_string(),
            ));
        }

        if election_timeout_max > 60000 {
            // 60 seconds max
            return Err(ConfluxError::validation(
                "Election timeout max cannot exceed 60000ms".to_string(),
            ));
        }

        debug!("Election timeout max {}ms is valid", election_timeout_max);
        Ok(())
    }

    /// 验证超时值之间的关系
    ///
    /// 确保心跳间隔小于选举超时，选举超时最小值小于最大值
    ///
    /// # Arguments
    ///
    /// * `heartbeat_interval` - 可选的心跳间隔
    /// * `election_timeout_min` - 可选的选举超时最小值
    /// * `election_timeout_max` - 可选的选举超时最大值
    ///
    /// # Returns
    ///
    /// 如果关系合理返回Ok(())，否则返回错误
    pub fn validate_timeout_relationships(
        &self,
        heartbeat_interval: Option<u64>,
        election_timeout_min: Option<u64>,
        election_timeout_max: Option<u64>,
    ) -> Result<()> {
        debug!("Validating timeout relationships");

        // 验证心跳间隔与选举超时最小值的关系
        if let (Some(heartbeat), Some(min_timeout)) = (heartbeat_interval, election_timeout_min) {
            if heartbeat >= min_timeout {
                return Err(ConfluxError::validation(
                    "Heartbeat interval must be less than election timeout min".to_string(),
                ));
            }

            // 推荐心跳间隔应该是选举超时的1/10到1/5
            let recommended_max = min_timeout / 5;
            let recommended_min = min_timeout / 10;

            if heartbeat > recommended_max {
                debug!(
                    "Warning: Heartbeat interval {}ms is high relative to election timeout {}ms (recommended: {}-{}ms)",
                    heartbeat, min_timeout, recommended_min, recommended_max
                );
            }
        }

        // 验证心跳间隔与选举超时最大值的关系
        if let (Some(heartbeat), Some(max_timeout)) = (heartbeat_interval, election_timeout_max) {
            if heartbeat >= max_timeout {
                return Err(ConfluxError::validation(
                    "Heartbeat interval must be less than election timeout max".to_string(),
                ));
            }
        }

        // 验证选举超时最小值与最大值的关系
        if let (Some(min_timeout), Some(max_timeout)) = (election_timeout_min, election_timeout_max)
        {
            if min_timeout >= max_timeout {
                return Err(ConfluxError::validation(
                    "Election timeout min must be less than max".to_string(),
                ));
            }

            // 推荐最大值应该是最小值的1.5-3倍
            let recommended_max = min_timeout * 3;
            let recommended_min = min_timeout * 3 / 2;

            if max_timeout > recommended_max {
                debug!(
                    "Warning: Election timeout max {}ms is high relative to min {}ms (recommended: {}-{}ms)",
                    max_timeout, min_timeout, recommended_min, recommended_max
                );
            }
        }

        debug!("Timeout relationships are valid");
        Ok(())
    }

    /// 推荐的超时配置
    ///
    /// 根据网络延迟推荐合适的超时配置
    ///
    /// # Arguments
    ///
    /// * `network_latency_ms` - 网络延迟（毫秒）
    ///
    /// # Returns
    ///
    /// 返回推荐的(心跳间隔, 选举超时最小值, 选举超时最大值)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use conflux::raft::validation::TimeoutValidator;
    ///
    /// let validator = TimeoutValidator::new();
    /// let (heartbeat, min_timeout, max_timeout) = validator.recommend_timeouts(10);
    ///
    /// assert!(heartbeat < min_timeout);
    /// assert!(min_timeout < max_timeout);
    /// ```
    pub fn recommend_timeouts(&self, network_latency_ms: u64) -> (u64, u64, u64) {
        // 心跳间隔应该是网络延迟的2-5倍
        let heartbeat_interval = std::cmp::max(50, network_latency_ms * 3);

        // 选举超时最小值应该是心跳间隔的5-10倍
        let election_timeout_min = heartbeat_interval * 7;

        // 选举超时最大值应该是最小值的2倍
        let election_timeout_max = election_timeout_min * 2;

        debug!(
            "Recommended timeouts for {}ms network latency: heartbeat={}ms, election_min={}ms, election_max={}ms",
            network_latency_ms, heartbeat_interval, election_timeout_min, election_timeout_max
        );

        (
            heartbeat_interval,
            election_timeout_min,
            election_timeout_max,
        )
    }

    /// 验证超时配置是否适合网络环境
    ///
    /// # Arguments
    ///
    /// * `heartbeat_interval` - 心跳间隔
    /// * `election_timeout_min` - 选举超时最小值
    /// * `network_latency_ms` - 网络延迟
    ///
    /// # Returns
    ///
    /// 如果配置适合网络环境返回Ok(())，否则返回警告
    pub fn validate_for_network(
        &self,
        heartbeat_interval: u64,
        election_timeout_min: u64,
        network_latency_ms: u64,
    ) -> Result<()> {
        debug!(
            "Validating timeouts for network: heartbeat={}ms, election_min={}ms, latency={}ms",
            heartbeat_interval, election_timeout_min, network_latency_ms
        );

        // 心跳间隔应该大于网络延迟的2倍
        if heartbeat_interval < network_latency_ms * 2 {
            return Err(ConfluxError::validation(format!(
                "Heartbeat interval {}ms is too small for network latency {}ms (recommend at least {}ms)",
                heartbeat_interval, network_latency_ms, network_latency_ms * 2
            )));
        }

        // 选举超时应该大于心跳间隔的5倍
        if election_timeout_min < heartbeat_interval * 5 {
            return Err(ConfluxError::validation(format!(
                "Election timeout min {}ms is too small for heartbeat interval {}ms (recommend at least {}ms)",
                election_timeout_min, heartbeat_interval, heartbeat_interval * 5
            )));
        }

        debug!("Timeout configuration is suitable for the network environment");
        Ok(())
    }
}

impl Default for TimeoutValidator {
    fn default() -> Self {
        Self::new()
    }
}
