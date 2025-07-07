use crate::protocol::http::{AppState, CreateVersionRequest, UpdateReleasesRequest, FetchConfigResponse};
use crate::raft::types::*;
use crate::raft::client::helpers::{create_write_request, create_get_config_request};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tracing::{debug, error, info};

/// 创建配置版本处理器
/// POST /api/v1/configs/{tenant}/{app}/{env}/{name}/versions
pub async fn create_version_handler(
    Path((tenant, app, env, name)): Path<(String, String, String, String)>,
    State(app_state): State<AppState>,
    Json(request): Json<CreateVersionRequest>,
) -> Result<Json<Value>, StatusCode> {
    info!("Creating version for config: {}/{}/{}/{}", tenant, app, env, name);

    let namespace = ConfigNamespace { tenant, app, env };

    // 首先需要找到配置的ID
    let config = match app_state.core_handle.store().get_config(&namespace, &name).await {
        Some(config) => config,
        None => {
            error!("Config not found: {}/{}/{}/{}", namespace.tenant, namespace.app, namespace.env, name);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // 创建 Raft 命令
    let command = RaftCommand::CreateVersion {
        config_id: config.id,
        content: request.content.into_bytes(),
        format: request.format,
        creator_id: request.creator_id.unwrap_or_else(|| "system".to_string()).parse().unwrap_or(0),
        description: request.description.unwrap_or_else(|| "Created via API".to_string()),
    };

    // 提交到 Raft
    let write_request = create_write_request(command);
    match app_state.core_handle.raft_client().write(write_request).await {
        Ok(response) => {
            info!("Version created successfully for {}/{}/{}/{}", namespace.tenant, namespace.app, namespace.env, name);
            Ok(Json(json!({
                "success": true,
                "data": response.data,
                "message": "Version created successfully"
            })))
        }
        Err(e) => {
            error!("Failed to create version: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 更新发布规则处理器
/// PUT /api/v1/configs/{tenant}/{app}/{env}/{name}/releases
pub async fn update_releases_handler(
    Path((tenant, app, env, name)): Path<(String, String, String, String)>,
    State(app_state): State<AppState>,
    Json(request): Json<UpdateReleasesRequest>,
) -> Result<Json<Value>, StatusCode> {
    info!("Updating releases for config: {}/{}/{}/{}", tenant, app, env, name);

    let namespace = ConfigNamespace { tenant, app, env };

    // 首先需要找到配置的ID
    let config = match app_state.core_handle.store().get_config(&namespace, &name).await {
        Some(config) => config,
        None => {
            error!("Config not found: {}/{}/{}/{}", namespace.tenant, namespace.app, namespace.env, name);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // 创建 Raft 命令
    let command = RaftCommand::UpdateReleaseRules {
        config_id: config.id,
        releases: request.releases,
    };

    // 提交到 Raft
    let write_request = create_write_request(command);
    match app_state.core_handle.raft_client().write(write_request).await {
        Ok(response) => {
            info!("Releases updated successfully for {}/{}/{}/{}", namespace.tenant, namespace.app, namespace.env, name);
            Ok(Json(json!({
                "success": true,
                "data": response.data,
                "message": "Releases updated successfully"
            })))
        }
        Err(e) => {
            error!("Failed to update releases: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取发布配置处理器
/// GET /api/v1/fetch/configs/{tenant}/{app}/{env}/{name}
pub async fn fetch_config_handler(
    Path((tenant, app, env, name)): Path<(String, String, String, String)>,
    Query(params): Query<BTreeMap<String, String>>,
    State(app_state): State<AppState>,
) -> Result<Json<FetchConfigResponse>, StatusCode> {
    debug!("Fetching config: {}/{}/{}/{} with labels: {:?}", tenant, app, env, name, params);

    let namespace = ConfigNamespace { tenant, app, env };
    
    // 创建读取请求
    let read_request = create_get_config_request(namespace.clone(), name.clone(), params);
    
    match app_state.core_handle.raft_client().read(read_request).await {
        Ok(response) => {
            if let Some(data) = response.data {
                // 解析返回的数据
                if let Ok(config_data) = serde_json::from_value::<serde_json::Value>(data) {
                    if let (Some(config), Some(version)) = (
                        config_data.get("config"),
                        config_data.get("version")
                    ) {
                        let fetch_response = FetchConfigResponse {
                            namespace: namespace.clone(),
                            name: name.clone(),
                            content: version.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            format: config.get("format").and_then(|v| v.as_str()).and_then(|s| {
                                match s {
                                    "Json" => Some(ConfigFormat::Json),
                                    "Yaml" => Some(ConfigFormat::Yaml),
                                    "Toml" => Some(ConfigFormat::Toml),
                                    "Properties" => Some(ConfigFormat::Properties),
                                    "Xml" => Some(ConfigFormat::Xml),
                                    _ => None,
                                }
                            }).unwrap_or(ConfigFormat::Json),
                            version_id: version.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                            hash: version.get("hash").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            created_at: chrono::Utc::now(), // TODO: 从实际数据中获取
                        };
                        
                        info!("Config fetched successfully: {}/{}/{}/{}", namespace.tenant, namespace.app, namespace.env, name);
                        return Ok(Json(fetch_response));
                    }
                }
            }
            
            error!("Config not found: {}/{}/{}/{}", namespace.tenant, namespace.app, namespace.env, name);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to fetch config: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取配置元数据处理器
/// GET /api/v1/configs/{tenant}/{app}/{env}/{name}
pub async fn get_config_handler(
    Path((tenant, app, env, name)): Path<(String, String, String, String)>,
    State(app_state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    debug!("Getting config metadata: {}/{}/{}/{}", tenant, app, env, name);

    let namespace = ConfigNamespace { tenant, app, env };

    // 直接从存储中读取配置元数据
    match app_state.core_handle.store().get_config(&namespace, &name).await {
        Some(config) => {
            info!("Config metadata retrieved: {}/{}/{}/{}", namespace.tenant, namespace.app, namespace.env, name);
            Ok(Json(json!(config)))
        }
        None => {
            debug!("Config not found: {}/{}/{}/{}", namespace.tenant, namespace.app, namespace.env, name);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// 列出配置版本处理器
/// GET /api/v1/configs/{tenant}/{app}/{env}/{name}/versions
pub async fn list_versions_handler(
    Path((tenant, app, env, name)): Path<(String, String, String, String)>,
    State(app_state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    debug!("Listing versions for config: {}/{}/{}/{}", tenant, app, env, name);

    let namespace = ConfigNamespace { tenant, app, env };

    // 首先需要找到配置的ID
    let config = match app_state.core_handle.store().get_config(&namespace, &name).await {
        Some(config) => config,
        None => {
            debug!("Config not found: {}/{}/{}/{}", namespace.tenant, namespace.app, namespace.env, name);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // 从存储中获取配置版本列表
    let versions = app_state.core_handle.store().list_config_versions(config.id).await;
    info!("Listed {} versions for config: {}/{}/{}/{}", versions.len(), namespace.tenant, namespace.app, namespace.env, name);
    Ok(Json(json!({
        "versions": versions,
        "count": versions.len()
    })))
}

/// 集群状态处理器
/// GET /_cluster/status
pub async fn cluster_status_handler(
    State(app_state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    debug!("Getting cluster status");

    match app_state.core_handle.raft_client().get_cluster_status().await {
        Ok(status) => {
            debug!("Cluster status retrieved successfully");
            Ok(Json(json!(status)))
        }
        Err(e) => {
            error!("Failed to get cluster status: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 添加节点处理器
/// POST /_cluster/nodes
pub async fn add_node_handler(
    State(_app_state): State<AppState>,
    Json(_request): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: 实现添加节点逻辑
    info!("Add node request received (not implemented yet)");
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// 移除节点处理器
/// DELETE /_cluster/nodes/{node_id}
pub async fn remove_node_handler(
    Path(_node_id): Path<u64>,
    State(_app_state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: 实现移除节点逻辑
    info!("Remove node request received (not implemented yet)");
    Err(StatusCode::NOT_IMPLEMENTED)
}
