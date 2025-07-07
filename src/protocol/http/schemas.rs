use crate::raft::types::{ConfigFormat, ConfigNamespace, Release};
use serde::{Deserialize, Serialize};

/// 创建配置版本请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    /// 配置内容
    pub content: String,
    /// 配置格式（可选，如果不提供则继承配置的默认格式）
    pub format: Option<ConfigFormat>,
    /// 创建者ID（可选，默认为 "system"）
    pub creator_id: Option<String>,
    /// 版本描述（可选）
    pub description: Option<String>,
}

/// 更新发布规则请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReleasesRequest {
    /// 新的发布规则列表
    pub releases: Vec<Release>,
    /// 更新者ID（可选，默认为 "system"）
    pub updater_id: Option<String>,
}

/// 获取配置响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchConfigResponse {
    /// 配置命名空间
    pub namespace: ConfigNamespace,
    /// 配置名称
    pub name: String,
    /// 配置内容
    pub content: String,
    /// 配置格式
    pub format: ConfigFormat,
    /// 版本ID
    pub version_id: u64,
    /// 内容哈希
    pub hash: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 通用API响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// 操作是否成功
    pub success: bool,
    /// 响应数据
    pub data: Option<T>,
    /// 响应消息
    pub message: Option<String>,
    /// 错误信息（当 success 为 false 时）
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    /// 创建成功响应
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            error: None,
        }
    }

    /// 创建成功响应（带消息）
    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
            error: None,
        }
    }

    /// 创建错误响应
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(error),
        }
    }
}

/// 分页查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// 页码（从1开始）
    pub page: Option<u32>,
    /// 每页大小
    pub page_size: Option<u32>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
        }
    }
}

/// 分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// 数据列表
    pub items: Vec<T>,
    /// 当前页码
    pub page: u32,
    /// 每页大小
    pub page_size: u32,
    /// 总数量
    pub total: u64,
    /// 总页数
    pub total_pages: u32,
}

impl<T> PaginatedResponse<T> {
    /// 创建分页响应
    pub fn new(items: Vec<T>, page: u32, page_size: u32, total: u64) -> Self {
        let total_pages = ((total as f64) / (page_size as f64)).ceil() as u32;
        Self {
            items,
            page,
            page_size,
            total,
            total_pages,
        }
    }
}

/// 配置查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigQueryParams {
    /// 配置名称前缀过滤
    pub prefix: Option<String>,
    /// 分页参数
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// 版本查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionQueryParams {
    /// 版本ID过滤
    pub version_id: Option<u64>,
    /// 创建者过滤
    pub creator_id: Option<String>,
    /// 分页参数
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// 健康检查响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// 服务状态
    pub status: String,
    /// 时间戳
    pub timestamp: String,
    /// 版本信息
    pub version: Option<String>,
    /// 额外信息
    pub details: Option<serde_json::Value>,
}

/// 集群节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// 节点ID
    pub id: u64,
    /// 节点地址
    pub address: String,
    /// 节点状态
    pub status: String,
    /// 是否为领导者
    pub is_leader: bool,
    /// 最后心跳时间
    pub last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
}

/// 添加节点请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddNodeRequest {
    /// 节点ID
    pub node_id: u64,
    /// 节点地址
    pub address: String,
}

/// 移除节点请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveNodeRequest {
    /// 节点ID
    pub node_id: u64,
    /// 是否强制移除
    pub force: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_create_version_request_serialization() {
        let request = CreateVersionRequest {
            content: "test content".to_string(),
            format: Some(ConfigFormat::Json),
            creator_id: Some("user123".to_string()),
            description: Some("Test version".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CreateVersionRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.content, deserialized.content);
        assert_eq!(request.format, deserialized.format);
        assert_eq!(request.creator_id, deserialized.creator_id);
        assert_eq!(request.description, deserialized.description);
    }

    #[test]
    fn test_api_response_creation() {
        let success_response = ApiResponse::success("test data".to_string());
        assert!(success_response.success);
        assert_eq!(success_response.data, Some("test data".to_string()));

        let error_response = ApiResponse::<String>::error("test error".to_string());
        assert!(!error_response.success);
        assert_eq!(error_response.error, Some("test error".to_string()));
    }

    #[test]
    fn test_paginated_response() {
        let items = vec!["item1".to_string(), "item2".to_string()];
        let response = PaginatedResponse::new(items.clone(), 1, 10, 25);

        assert_eq!(response.items, items);
        assert_eq!(response.page, 1);
        assert_eq!(response.page_size, 10);
        assert_eq!(response.total, 25);
        assert_eq!(response.total_pages, 3);
    }

    #[test]
    fn test_update_releases_request() {
        let releases = vec![Release::new(BTreeMap::new(), 1, 0)];
        let request = UpdateReleasesRequest {
            releases: releases.clone(),
            updater_id: Some("admin".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: UpdateReleasesRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.releases.len(), deserialized.releases.len());
        assert_eq!(request.updater_id, deserialized.updater_id);
    }
}
