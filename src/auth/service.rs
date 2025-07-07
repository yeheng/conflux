use casbin::{CoreApi, Enforcer, MgmtApi, RbacApi};
use sqlx_adapter::SqlxAdapter;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::error::{ConfluxError, Result};

/// 认证授权服务
/// 
/// 封装了Casbin Enforcer，提供更符合业务的接口
#[derive(Clone)]
pub struct AuthzService {
    enforcer: Arc<RwLock<Enforcer>>,
}

impl AuthzService {
    /// 创建一个新的AuthzService实例
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL数据库连接字符串
    ///
    /// # Returns
    /// * `Result<Self>` - 成功时返回AuthzService实例
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Initializing AuthzService with Casbin");

        // 创建SqlxAdapter
        let adapter = SqlxAdapter::new(database_url, 8)
            .await
            .map_err(|e| {
                error!("Failed to create SqlxAdapter: {}", e);
                ConfluxError::AuthError(format!("Failed to create SqlxAdapter: {}", e))
            })?;

        // 创建Enforcer，使用model.conf文件
        let model_path = "src/auth/model.conf";
        let mut enforcer = Enforcer::new(model_path, adapter)
            .await
            .map_err(|e| {
                error!("Failed to create Casbin Enforcer: {}", e);
                ConfluxError::AuthError(format!("Failed to create Casbin Enforcer: {}", e))
            })?;

        // 构建角色链接，对于RBAC模型是必须的
        enforcer.build_role_links().map_err(|e| {
            error!("Failed to build role links: {}", e);
            ConfluxError::AuthError(format!("Failed to build role links: {}", e))
        })?;

        info!("AuthzService initialized successfully");
        
        Ok(Self {
            enforcer: Arc::new(RwLock::new(enforcer)),
        })
    }

    /// 核心检查函数：检查一个用户在特定租户下是否有权对资源执行操作
    /// 
    /// # Arguments
    /// * `user_id` - 发起请求的用户唯一标识
    /// * `tenant` - 请求所属的租户
    /// * `resource` - 被访问的资源路径，例如 "/apps/my-app/envs/prod/configs/db.toml"
    /// * `action` - 执行的操作，例如 "read", "write"
    /// 
    /// # Returns
    /// * `Result<bool>` - 是否有权限
    pub async fn check(
        &self,
        user_id: &str,
        tenant: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool> {
        debug!(
            "Checking permission: user={}, tenant={}, resource={}, action={}",
            user_id, tenant, resource, action
        );

        let enforcer = self.enforcer.read().await;
        let result = enforcer
            .enforce((user_id, tenant, resource, action))
            .map_err(|e| {
                error!("Permission check failed: {}", e);
                ConfluxError::AuthError(format!("Permission check failed: {}", e))
            })?;

        debug!(
            "Permission check result: user={}, tenant={}, resource={}, action={}, allowed={}",
            user_id, tenant, resource, action, result
        );

        Ok(result)
    }

    /// 为角色添加权限
    /// 
    /// # Arguments
    /// * `role` - 角色名称
    /// * `tenant` - 租户ID
    /// * `resource` - 资源路径模式，支持通配符
    /// * `action` - 操作类型
    /// 
    /// # Returns
    /// * `Result<bool>` - 是否成功添加
    pub async fn add_permission_for_role(
        &self,
        role: &str,
        tenant: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool> {
        info!(
            "Adding permission: role={}, tenant={}, resource={}, action={}",
            role, tenant, resource, action
        );

        let mut enforcer = self.enforcer.write().await;
        let rules = vec![vec![
            role.to_string(),
            tenant.to_string(),
            resource.to_string(),
            action.to_string(),
        ]];

        let result = enforcer.add_policies(rules).await.map_err(|e| {
            error!("Failed to add permission: {}", e);
            ConfluxError::AuthError(format!("Failed to add permission: {}", e))
        })?;

        info!(
            "Permission added successfully: role={}, tenant={}, resource={}, action={}",
            role, tenant, resource, action
        );

        Ok(result)
    }

    /// 移除角色的权限
    /// 
    /// # Arguments
    /// * `role` - 角色名称
    /// * `tenant` - 租户ID
    /// * `resource` - 资源路径模式
    /// * `action` - 操作类型
    /// 
    /// # Returns
    /// * `Result<bool>` - 是否成功移除
    pub async fn remove_permission_for_role(
        &self,
        role: &str,
        tenant: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool> {
        info!(
            "Removing permission: role={}, tenant={}, resource={}, action={}",
            role, tenant, resource, action
        );

        let mut enforcer = self.enforcer.write().await;
        let rules = vec![vec![
            role.to_string(),
            tenant.to_string(),
            resource.to_string(),
            action.to_string(),
        ]];

        let result = enforcer.remove_policies(rules).await.map_err(|e| {
            error!("Failed to remove permission: {}", e);
            ConfluxError::AuthError(format!("Failed to remove permission: {}", e))
        })?;

        info!(
            "Permission removed successfully: role={}, tenant={}, resource={}, action={}",
            role, tenant, resource, action
        );

        Ok(result)
    }

    /// 为用户分配角色
    /// 
    /// # Arguments
    /// * `user_id` - 用户ID
    /// * `role` - 角色名称
    /// * `tenant` - 租户ID
    /// 
    /// # Returns
    /// * `Result<bool>` - 是否成功分配
    pub async fn assign_role_to_user(
        &self,
        user_id: &str,
        role: &str,
        tenant: &str,
    ) -> Result<bool> {
        info!(
            "Assigning role to user: user={}, role={}, tenant={}",
            user_id, role, tenant
        );

        let mut enforcer = self.enforcer.write().await;
        let result = enforcer
            .add_role_for_user(user_id, role, Some(tenant))
            .await
            .map_err(|e| {
                error!("Failed to assign role: {}", e);
                ConfluxError::AuthError(format!("Failed to assign role: {}", e))
            })?;

        info!(
            "Role assigned successfully: user={}, role={}, tenant={}",
            user_id, role, tenant
        );

        Ok(result)
    }

    /// 撤销用户的角色
    /// 
    /// # Arguments
    /// * `user_id` - 用户ID
    /// * `role` - 角色名称
    /// * `tenant` - 租户ID
    /// 
    /// # Returns
    /// * `Result<bool>` - 是否成功撤销
    pub async fn revoke_role_from_user(
        &self,
        user_id: &str,
        role: &str,
        tenant: &str,
    ) -> Result<bool> {
        info!(
            "Revoking role from user: user={}, role={}, tenant={}",
            user_id, role, tenant
        );

        let mut enforcer = self.enforcer.write().await;
        let result = enforcer
            .delete_role_for_user(user_id, role, Some(tenant))
            .await
            .map_err(|e| {
                error!("Failed to revoke role: {}", e);
                ConfluxError::AuthError(format!("Failed to revoke role: {}", e))
            })?;

        info!(
            "Role revoked successfully: user={}, role={}, tenant={}",
            user_id, role, tenant
        );

        Ok(result)
    }

    /// 获取用户在租户下的所有角色
    /// 
    /// # Arguments
    /// * `user_id` - 用户ID
    /// * `tenant` - 租户ID
    /// 
    /// # Returns
    /// * `Result<Vec<String>>` - 角色列表
    pub async fn get_roles_for_user_in_tenant(
        &self,
        user_id: &str,
        tenant: &str,
    ) -> Result<Vec<String>> {
        debug!("Getting roles for user: user={}, tenant={}", user_id, tenant);

        let enforcer = self.enforcer.read().await;
        let roles = enforcer.get_roles_for_user(user_id, Some(tenant));

        debug!(
            "Found roles for user: user={}, tenant={}, roles={:?}",
            user_id, tenant, roles
        );

        Ok(roles)
    }

    /// 重新加载策略（用于热更新）
    /// 
    /// # Returns
    /// * `Result<()>` - 是否成功重新加载
    pub async fn reload_policy(&self) -> Result<()> {
        info!("Reloading Casbin policies");

        let mut enforcer = self.enforcer.write().await;
        enforcer.load_policy().await.map_err(|e| {
            error!("Failed to reload policy: {}", e);
            ConfluxError::AuthError(format!("Failed to reload policy: {}", e))
        })?;

        // 重新构建角色链接
        enforcer.build_role_links().map_err(|e| {
            error!("Failed to rebuild role links: {}", e);
            ConfluxError::AuthError(format!("Failed to rebuild role links: {}", e))
        })?;

        info!("Casbin policies reloaded successfully");
        Ok(())
    }
}
