use crate::app::CoreAppHandle;
use crate::protocol::{ProtocolConfig, ProtocolPlugin};
use async_trait::async_trait;
use axum::{
    extract::State,
    http::StatusCode,
    middleware::from_fn,
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};

pub mod handlers;
pub mod middleware;
pub mod schemas;

pub use handlers::*;
pub use middleware::logging_middleware;
pub use schemas::*;

/// HTTP 协议插件实现
pub struct HttpProtocol;

#[async_trait]
impl ProtocolPlugin for HttpProtocol {
    fn name(&self) -> &'static str {
        "http-rest"
    }

    async fn start(&self, core_handle: CoreAppHandle, config: ProtocolConfig) -> anyhow::Result<()> {
        info!("Starting HTTP protocol plugin on {}", config.listen_addr);

        // 创建应用状态
        let app_state = AppState::new(core_handle);

        // 构建路由
        let app = create_router(app_state);

        // 解析监听地址
        let addr: SocketAddr = config.listen_addr.parse()
            .map_err(|e| anyhow::anyhow!("Invalid listen address: {}", e))?;

        // 启动服务器
        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("HTTP server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }

    async fn health_check(&self) -> bool {
        // TODO: 实现更复杂的健康检查逻辑
        true
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        info!("Shutting down HTTP protocol plugin");
        Ok(())
    }
}

/// 应用状态，包含核心服务的引用
#[derive(Clone)]
pub struct AppState {
    pub core_handle: CoreAppHandle,
}

impl AppState {
    pub fn new(core_handle: CoreAppHandle) -> Self {
        Self { core_handle }
    }
}

/// 创建 Axum 路由器
fn create_router(app_state: AppState) -> Router {
    Router::new()
        // 健康检查端点（公共访问）
        .route("/health", get(health_handler))
        .route("/ready", get(readiness_handler))

        // API v1 路由（暂时不添加授权中间件）
        .nest("/api/v1", create_v1_routes())

        // 集群管理路由
        .nest("/_cluster", create_cluster_routes())

        // 设置应用状态
        .with_state(app_state)

        // 添加全局中间件
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                // 添加请求日志中间件
                .layer(from_fn(logging_middleware))
        )
}

/// 创建 API v1 路由
fn create_v1_routes() -> Router<AppState> {
    Router::new()
        // 配置管理路由
        .route("/configs/{tenant}/{app}/{env}/{name}/versions", post(create_version_handler))
        .route("/configs/{tenant}/{app}/{env}/{name}/releases", put(update_releases_handler))
        .route("/fetch/configs/{tenant}/{app}/{env}/{name}", get(fetch_config_handler))

        // 配置查询路由
        .route("/configs/{tenant}/{app}/{env}/{name}", get(get_config_handler))
        .route("/configs/{tenant}/{app}/{env}/{name}/versions", get(list_versions_handler))
}

/// 创建集群管理路由
fn create_cluster_routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(cluster_status_handler))
        .route("/nodes", post(add_node_handler))
        .route("/nodes/{node_id}", axum::routing::delete(remove_node_handler))
}

/// 健康检查处理器
async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// 就绪检查处理器
async fn readiness_handler(State(app_state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    // 检查核心服务是否就绪
    let cluster_status = app_state.core_handle.raft_client()
        .get_cluster_status()
        .await;

    match cluster_status {
        Ok(_) => Ok(Json(json!({
            "status": "ready",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))),
        Err(e) => {
            warn!("Readiness check failed: {}", e);
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

// TODO: 更新测试以包含AuthzService
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::raft::{RaftClient, Store};
//     use std::sync::Arc;
//     use tempfile::TempDir;

//     #[tokio::test]
//     async fn test_http_protocol_creation() {
//         let protocol = HttpProtocol;
//         assert_eq!(protocol.name(), "http-rest");
//         assert!(protocol.health_check().await);
//     }

//     #[tokio::test]
//     async fn test_app_state_creation() {
//         let temp_dir = TempDir::new().unwrap();
//         let store = Arc::new(Store::new(temp_dir.path()).await.unwrap());
//         let raft_client = Arc::new(RaftClient::new(store.clone()));
//         let core_handle = CoreAppHandle::new(raft_client, store);

//         let app_state = AppState::new(core_handle);

//         // 验证状态创建成功
//         assert!(Arc::ptr_eq(&app_state.core_handle.raft_client, &app_state.core_handle.raft_client));
//     }

//     #[tokio::test]
//     async fn test_router_creation() {
//         let temp_dir = TempDir::new().unwrap();
//         let store = Arc::new(Store::new(temp_dir.path()).await.unwrap());
//         let raft_client = Arc::new(RaftClient::new(store.clone()));
//         let core_handle = CoreAppHandle::new(raft_client, store);
//         let app_state = AppState::new(core_handle);

//         let _router = create_router(app_state);
//         // 如果能创建路由器而不出错，测试就通过了
//     }
// }
