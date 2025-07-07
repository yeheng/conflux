use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

use super::AuthzService;

/// 权限检查请求
#[derive(Debug, Deserialize)]
pub struct CheckPermissionRequest {
    pub user_id: String,
    pub tenant: String,
    pub resource: String,
    pub action: String,
}

/// 权限检查响应
#[derive(Debug, Serialize)]
pub struct CheckPermissionResponse {
    pub allowed: bool,
    pub user_id: String,
    pub tenant: String,
    pub resource: String,
    pub action: String,
}

/// 添加权限请求
#[derive(Debug, Deserialize)]
pub struct AddPermissionRequest {
    pub resource: String,
    pub action: String,
}

/// 角色分配请求
#[derive(Debug, Deserialize)]
pub struct AssignRoleRequest {
    pub role: String,
}

/// 角色列表响应
#[derive(Debug, Serialize)]
pub struct RolesResponse {
    pub roles: Vec<String>,
}

/// 操作结果响应
#[derive(Debug, Serialize)]
pub struct OperationResponse {
    pub success: bool,
    pub message: String,
}

/// 创建认证授权相关的路由
pub fn create_auth_routes<S>(authz_service: Arc<AuthzService>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        // 权限检查端点（主要用于调试）
        .route("/_auth/check", post(check_permission))
        // 租户角色管理
        .route("/tenants/:tenant/roles", get(list_tenant_roles))
        // 角色权限管理
        .route(
            "/tenants/:tenant/roles/:role/permissions",
            post(add_role_permission),
        )
        .route(
            "/tenants/:tenant/roles/:role/permissions",
            delete(remove_role_permission),
        )
        // 用户角色管理
        .route(
            "/tenants/:tenant/users/:user_id/roles",
            get(get_user_roles),
        )
        .route(
            "/tenants/:tenant/users/:user_id/roles",
            post(assign_user_role),
        )
        .route(
            "/tenants/:tenant/users/:user_id/roles/:role",
            delete(revoke_user_role),
        )
        // 策略重新加载
        .route("/_auth/reload", post(reload_policies))
        .with_state(authz_service)
}

/// 检查权限（调试用）
async fn check_permission(
    State(authz_service): State<Arc<AuthzService>>,
    Json(request): Json<CheckPermissionRequest>,
) -> std::result::Result<Json<CheckPermissionResponse>, StatusCode> {
    debug!("Checking permission: {:?}", request);

    let allowed = authz_service
        .check(&request.user_id, &request.tenant, &request.resource, &request.action)
        .await
        .map_err(|e| {
            tracing::error!("Permission check failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(CheckPermissionResponse {
        allowed,
        user_id: request.user_id,
        tenant: request.tenant,
        resource: request.resource,
        action: request.action,
    }))
}

/// 列出租户下的所有角色（占位符实现）
async fn list_tenant_roles(
    Path(tenant): Path<String>,
    State(_authz_service): State<Arc<AuthzService>>,
) -> std::result::Result<Json<RolesResponse>, StatusCode> {
    debug!("Listing roles for tenant: {}", tenant);

    // TODO: 实现从Casbin中获取租户的所有角色
    // 目前返回一些预定义的角色
    let roles = vec![
        "admin".to_string(),
        "developer".to_string(),
        "viewer".to_string(),
    ];

    Ok(Json(RolesResponse { roles }))
}

/// 为角色添加权限
async fn add_role_permission(
    Path((tenant, role)): Path<(String, String)>,
    State(authz_service): State<Arc<AuthzService>>,
    Json(request): Json<AddPermissionRequest>,
) -> std::result::Result<Json<OperationResponse>, StatusCode> {
    info!(
        "Adding permission to role: tenant={}, role={}, resource={}, action={}",
        tenant, role, request.resource, request.action
    );

    let success = authz_service
        .add_permission_for_role(&role, &tenant, &request.resource, &request.action)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add permission: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let message = if success {
        "Permission added successfully".to_string()
    } else {
        "Permission already exists".to_string()
    };

    Ok(Json(OperationResponse { success, message }))
}

/// 移除角色的权限
async fn remove_role_permission(
    Path((tenant, role)): Path<(String, String)>,
    State(authz_service): State<Arc<AuthzService>>,
    Query(params): Query<HashMap<String, String>>,
) -> std::result::Result<Json<OperationResponse>, StatusCode> {
    let resource = params
        .get("resource")
        .ok_or(StatusCode::BAD_REQUEST)?
        .clone();
    let action = params.get("action").ok_or(StatusCode::BAD_REQUEST)?.clone();

    info!(
        "Removing permission from role: tenant={}, role={}, resource={}, action={}",
        tenant, role, resource, action
    );

    let success = authz_service
        .remove_permission_for_role(&role, &tenant, &resource, &action)
        .await
        .map_err(|e| {
            tracing::error!("Failed to remove permission: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let message = if success {
        "Permission removed successfully".to_string()
    } else {
        "Permission not found".to_string()
    };

    Ok(Json(OperationResponse { success, message }))
}

/// 获取用户在租户下的角色
async fn get_user_roles(
    Path((tenant, user_id)): Path<(String, String)>,
    State(authz_service): State<Arc<AuthzService>>,
) -> std::result::Result<Json<RolesResponse>, StatusCode> {
    debug!("Getting roles for user: tenant={}, user_id={}", tenant, user_id);

    let roles = authz_service
        .get_roles_for_user_in_tenant(&user_id, &tenant)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get user roles: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(RolesResponse { roles }))
}

/// 为用户分配角色
async fn assign_user_role(
    Path((tenant, user_id)): Path<(String, String)>,
    State(authz_service): State<Arc<AuthzService>>,
    Json(request): Json<AssignRoleRequest>,
) -> std::result::Result<Json<OperationResponse>, StatusCode> {
    info!(
        "Assigning role to user: tenant={}, user_id={}, role={}",
        tenant, user_id, request.role
    );

    let success = authz_service
        .assign_role_to_user(&user_id, &request.role, &tenant)
        .await
        .map_err(|e| {
            tracing::error!("Failed to assign role: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let message = if success {
        "Role assigned successfully".to_string()
    } else {
        "Role already assigned".to_string()
    };

    Ok(Json(OperationResponse { success, message }))
}

/// 撤销用户的角色
async fn revoke_user_role(
    Path((tenant, user_id, role)): Path<(String, String, String)>,
    State(authz_service): State<Arc<AuthzService>>,
) -> std::result::Result<Json<OperationResponse>, StatusCode> {
    info!(
        "Revoking role from user: tenant={}, user_id={}, role={}",
        tenant, user_id, role
    );

    let success = authz_service
        .revoke_role_from_user(&user_id, &role, &tenant)
        .await
        .map_err(|e| {
            tracing::error!("Failed to revoke role: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let message = if success {
        "Role revoked successfully".to_string()
    } else {
        "Role assignment not found".to_string()
    };

    Ok(Json(OperationResponse { success, message }))
}

/// 重新加载策略
async fn reload_policies(
    State(authz_service): State<Arc<AuthzService>>,
) -> std::result::Result<Json<OperationResponse>, StatusCode> {
    info!("Reloading Casbin policies");

    authz_service.reload_policy().await.map_err(|e| {
        tracing::error!("Failed to reload policies: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(OperationResponse {
        success: true,
        message: "Policies reloaded successfully".to_string(),
    }))
}
