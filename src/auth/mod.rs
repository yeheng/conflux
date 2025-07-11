//! 认证与授权模块
//! 
//! 基于Casbin实现的RBAC权限控制系统，支持多租户架构

pub mod api;
pub mod middleware;
pub mod service;

#[cfg(test)]
mod unit_tests;

pub use api::create_auth_routes;
pub use middleware::{authz_middleware, AuthzMiddleware};
pub use service::AuthzService;

/// 认证上下文
/// 
/// 包含从JWT或其他认证方式中提取的用户信息
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// 用户ID
    pub user_id: String,
    /// 租户ID
    pub tenant_id: String,
    /// 用户角色列表（可选，用于缓存）
    pub roles: Option<Vec<String>>,
}

impl AuthContext {
    /// 创建新的认证上下文
    pub fn new(user_id: String, tenant_id: String) -> Self {
        Self {
            user_id,
            tenant_id,
            roles: None,
        }
    }

    /// 创建带角色信息的认证上下文
    pub fn with_roles(user_id: String, tenant_id: String, roles: Vec<String>) -> Self {
        Self {
            user_id,
            tenant_id,
            roles: Some(roles),
        }
    }
}

/// 权限检查结果
#[derive(Debug, Clone)]
pub struct PermissionResult {
    /// 是否有权限
    pub allowed: bool,
    /// 检查的资源
    pub resource: String,
    /// 检查的操作
    pub action: String,
    /// 检查的用户
    pub user_id: String,
    /// 检查的租户
    pub tenant_id: String,
}

impl PermissionResult {
    /// 创建允许的权限结果
    pub fn allowed(user_id: String, tenant_id: String, resource: String, action: String) -> Self {
        Self {
            allowed: true,
            resource,
            action,
            user_id,
            tenant_id,
        }
    }

    /// 创建拒绝的权限结果
    pub fn denied(user_id: String, tenant_id: String, resource: String, action: String) -> Self {
        Self {
            allowed: false,
            resource,
            action,
            user_id,
            tenant_id,
        }
    }
}

/// 常用的操作类型
pub mod actions {
    pub const READ: &str = "read";
    pub const WRITE: &str = "write";
    pub const DELETE: &str = "delete";
    pub const ADMIN: &str = "admin";
    
    // Raft cluster operations
    pub const CLUSTER_ADD_NODE: &str = "cluster:add_node";
    pub const CLUSTER_REMOVE_NODE: &str = "cluster:remove_node";
    pub const CLUSTER_VIEW_METRICS: &str = "cluster:view_metrics";
    pub const CLUSTER_CHANGE_CONFIG: &str = "cluster:change_config";
    pub const CLUSTER_ADMIN: &str = "cluster:admin";
}

/// 常用的角色类型
pub mod roles {
    pub const SUPER_ADMIN: &str = "super_admin";
    pub const TENANT_ADMIN: &str = "tenant_admin";
    pub const DEVELOPER: &str = "developer";
    pub const VIEWER: &str = "viewer";
    
    // Raft cluster roles
    pub const CLUSTER_ADMIN: &str = "cluster_admin";
    pub const CLUSTER_OPERATOR: &str = "cluster_operator";
    pub const CLUSTER_VIEWER: &str = "cluster_viewer";
}

/// 资源路径构建器
pub struct ResourcePath;

impl ResourcePath {
    /// 构建配置资源路径
    pub fn config(tenant: &str, app: &str, env: &str, config_name: &str) -> String {
        format!("/tenants/{}/apps/{}/envs/{}/configs/{}", tenant, app, env, config_name)
    }

    /// 构建应用资源路径
    pub fn app(tenant: &str, app: &str) -> String {
        format!("/tenants/{}/apps/{}", tenant, app)
    }

    /// 构建环境资源路径
    pub fn env(tenant: &str, app: &str, env: &str) -> String {
        format!("/tenants/{}/apps/{}/envs/{}", tenant, app, env)
    }

    /// 构建租户资源路径
    pub fn tenant(tenant: &str) -> String {
        format!("/tenants/{}", tenant)
    }

    /// 构建管理资源路径
    pub fn admin(tenant: &str, resource: &str) -> String {
        format!("/tenants/{}/admin/{}", tenant, resource)
    }
    
    /// 构建集群资源路径
    pub fn cluster(tenant: &str) -> String {
        format!("/tenants/{}/cluster", tenant)
    }
    
    /// 构建集群节点资源路径
    pub fn cluster_node(tenant: &str, node_id: u64) -> String {
        format!("/tenants/{}/cluster/nodes/{}", tenant, node_id)
    }
    
    /// 构建集群指标资源路径
    pub fn cluster_metrics(tenant: &str) -> String {
        format!("/tenants/{}/cluster/metrics", tenant)
    }
    
    /// 构建集群配置资源路径
    pub fn cluster_config(tenant: &str) -> String {
        format!("/tenants/{}/cluster/config", tenant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_context_creation() {
        let ctx = AuthContext::new("user1".to_string(), "tenant1".to_string());
        assert_eq!(ctx.user_id, "user1");
        assert_eq!(ctx.tenant_id, "tenant1");
        assert!(ctx.roles.is_none());

        let ctx_with_roles = AuthContext::with_roles(
            "user1".to_string(),
            "tenant1".to_string(),
            vec!["admin".to_string()],
        );
        assert!(ctx_with_roles.roles.is_some());
        assert_eq!(ctx_with_roles.roles.unwrap(), vec!["admin"]);
    }

    #[test]
    fn test_permission_result() {
        let allowed = PermissionResult::allowed(
            "user1".to_string(),
            "tenant1".to_string(),
            "/resource".to_string(),
            "read".to_string(),
        );
        assert!(allowed.allowed);

        let denied = PermissionResult::denied(
            "user1".to_string(),
            "tenant1".to_string(),
            "/resource".to_string(),
            "write".to_string(),
        );
        assert!(!denied.allowed);
    }

    #[test]
    fn test_resource_path_builder() {
        assert_eq!(
            ResourcePath::config("tenant1", "app1", "prod", "db.toml"),
            "/tenants/tenant1/apps/app1/envs/prod/configs/db.toml"
        );

        assert_eq!(
            ResourcePath::app("tenant1", "app1"),
            "/tenants/tenant1/apps/app1"
        );

        assert_eq!(
            ResourcePath::tenant("tenant1"),
            "/tenants/tenant1"
        );
    }
}
