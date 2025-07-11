//! Raft cluster authorization module
//!
//! Provides RBAC authorization for Raft cluster operations
//! Integrates with the existing Casbin-based auth system

use crate::auth::{actions, roles, AuthContext, AuthzService, PermissionResult, ResourcePath};
use crate::error::{ConfluxError, Result};
use crate::raft::types::NodeId;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Raft cluster authorization service
/// 
/// Handles permission checks for cluster operations
#[derive(Clone)]
pub struct RaftAuthzService {
    /// Base authorization service
    authz_service: Arc<AuthzService>,
    /// Default tenant for cluster operations (if multi-tenant cluster)
    default_tenant: String,
}

impl RaftAuthzService {
    /// Create a new Raft authorization service
    pub fn new(authz_service: Arc<AuthzService>, default_tenant: String) -> Self {
        Self {
            authz_service,
            default_tenant,
        }
    }

    /// Check if user can add nodes to the cluster
    pub async fn check_add_node_permission(
        &self,
        auth_ctx: &AuthContext,
        node_id: NodeId,
    ) -> Result<PermissionResult> {
        let tenant = &auth_ctx.tenant_id;
        let resource = ResourcePath::cluster_node(tenant, node_id);
        
        debug!(
            "Checking add_node permission: user={}, tenant={}, node_id={}",
            auth_ctx.user_id, tenant, node_id
        );

        let allowed = self.authz_service
            .check(&auth_ctx.user_id, tenant, &resource, actions::CLUSTER_ADD_NODE)
            .await?;

        let result = if allowed {
            PermissionResult::allowed(
                auth_ctx.user_id.clone(),
                tenant.clone(),
                resource,
                actions::CLUSTER_ADD_NODE.to_string(),
            )
        } else {
            PermissionResult::denied(
                auth_ctx.user_id.clone(),
                tenant.clone(),
                resource,
                actions::CLUSTER_ADD_NODE.to_string(),
            )
        };

        if !allowed {
            warn!(
                "Permission denied for add_node: user={}, tenant={}, node_id={}",
                auth_ctx.user_id, tenant, node_id
            );
        }

        Ok(result)
    }

    /// Check if user can remove nodes from the cluster
    pub async fn check_remove_node_permission(
        &self,
        auth_ctx: &AuthContext,
        node_id: NodeId,
    ) -> Result<PermissionResult> {
        let tenant = &auth_ctx.tenant_id;
        let resource = ResourcePath::cluster_node(tenant, node_id);
        
        debug!(
            "Checking remove_node permission: user={}, tenant={}, node_id={}",
            auth_ctx.user_id, tenant, node_id
        );

        let allowed = self.authz_service
            .check(&auth_ctx.user_id, tenant, &resource, actions::CLUSTER_REMOVE_NODE)
            .await?;

        let result = if allowed {
            PermissionResult::allowed(
                auth_ctx.user_id.clone(),
                tenant.clone(),
                resource,
                actions::CLUSTER_REMOVE_NODE.to_string(),
            )
        } else {
            PermissionResult::denied(
                auth_ctx.user_id.clone(),
                tenant.clone(),
                resource,
                actions::CLUSTER_REMOVE_NODE.to_string(),
            )
        };

        if !allowed {
            warn!(
                "Permission denied for remove_node: user={}, tenant={}, node_id={}",
                auth_ctx.user_id, tenant, node_id
            );
        }

        Ok(result)
    }

    /// Check if user can view cluster metrics
    pub async fn check_view_metrics_permission(
        &self,
        auth_ctx: &AuthContext,
    ) -> Result<PermissionResult> {
        let tenant = &auth_ctx.tenant_id;
        let resource = ResourcePath::cluster_metrics(tenant);
        
        debug!(
            "Checking view_metrics permission: user={}, tenant={}",
            auth_ctx.user_id, tenant
        );

        let allowed = self.authz_service
            .check(&auth_ctx.user_id, tenant, &resource, actions::CLUSTER_VIEW_METRICS)
            .await?;

        let result = if allowed {
            PermissionResult::allowed(
                auth_ctx.user_id.clone(),
                tenant.clone(),
                resource,
                actions::CLUSTER_VIEW_METRICS.to_string(),
            )
        } else {
            PermissionResult::denied(
                auth_ctx.user_id.clone(),
                tenant.clone(),
                resource,
                actions::CLUSTER_VIEW_METRICS.to_string(),
            )
        };

        if !allowed {
            warn!(
                "Permission denied for view_metrics: user={}, tenant={}",
                auth_ctx.user_id, tenant
            );
        }

        Ok(result)
    }

    /// Check if user can change cluster configuration
    pub async fn check_change_config_permission(
        &self,
        auth_ctx: &AuthContext,
    ) -> Result<PermissionResult> {
        let tenant = &auth_ctx.tenant_id;
        let resource = ResourcePath::cluster_config(tenant);
        
        debug!(
            "Checking change_config permission: user={}, tenant={}",
            auth_ctx.user_id, tenant
        );

        let allowed = self.authz_service
            .check(&auth_ctx.user_id, tenant, &resource, actions::CLUSTER_CHANGE_CONFIG)
            .await?;

        let result = if allowed {
            PermissionResult::allowed(
                auth_ctx.user_id.clone(),
                tenant.clone(),
                resource,
                actions::CLUSTER_CHANGE_CONFIG.to_string(),
            )
        } else {
            PermissionResult::denied(
                auth_ctx.user_id.clone(),
                tenant.clone(),
                resource,
                actions::CLUSTER_CHANGE_CONFIG.to_string(),
            )
        };

        if !allowed {
            warn!(
                "Permission denied for change_config: user={}, tenant={}",
                auth_ctx.user_id, tenant
            );
        }

        Ok(result)
    }

    /// Check if user has admin privileges for the cluster
    pub async fn check_cluster_admin_permission(
        &self,
        auth_ctx: &AuthContext,
    ) -> Result<PermissionResult> {
        let tenant = &auth_ctx.tenant_id;
        let resource = ResourcePath::cluster(tenant);
        
        debug!(
            "Checking cluster_admin permission: user={}, tenant={}",
            auth_ctx.user_id, tenant
        );

        let allowed = self.authz_service
            .check(&auth_ctx.user_id, tenant, &resource, actions::CLUSTER_ADMIN)
            .await?;

        let result = if allowed {
            PermissionResult::allowed(
                auth_ctx.user_id.clone(),
                tenant.clone(),
                resource,
                actions::CLUSTER_ADMIN.to_string(),
            )
        } else {
            PermissionResult::denied(
                auth_ctx.user_id.clone(),
                tenant.clone(),
                resource,
                actions::CLUSTER_ADMIN.to_string(),
            )
        };

        if !allowed {
            warn!(
                "Permission denied for cluster_admin: user={}, tenant={}",
                auth_ctx.user_id, tenant
            );
        }

        Ok(result)
    }

    /// Initialize default cluster permissions
    /// 
    /// Sets up the basic role-permission mappings for cluster operations
    pub async fn initialize_cluster_permissions(&self, tenant: &str) -> Result<()> {
        info!("Initializing cluster permissions for tenant: {}", tenant);

        // Cluster admin permissions
        let cluster_resource = ResourcePath::cluster(tenant);
        let metrics_resource = ResourcePath::cluster_metrics(tenant);
        let config_resource = ResourcePath::cluster_config(tenant);
        let node_resource = ResourcePath::cluster_node(tenant, 0); // Wildcard pattern

        // Grant cluster_admin full access
        self.authz_service.add_permission_for_role(
            roles::CLUSTER_ADMIN, tenant, &cluster_resource, actions::CLUSTER_ADMIN
        ).await?;

        // Grant cluster_operator node management access
        self.authz_service.add_permission_for_role(
            roles::CLUSTER_OPERATOR, tenant, &node_resource, actions::CLUSTER_ADD_NODE
        ).await?;
        self.authz_service.add_permission_for_role(
            roles::CLUSTER_OPERATOR, tenant, &node_resource, actions::CLUSTER_REMOVE_NODE
        ).await?;
        self.authz_service.add_permission_for_role(
            roles::CLUSTER_OPERATOR, tenant, &config_resource, actions::CLUSTER_CHANGE_CONFIG
        ).await?;

        // Grant cluster_viewer metrics access
        self.authz_service.add_permission_for_role(
            roles::CLUSTER_VIEWER, tenant, &metrics_resource, actions::CLUSTER_VIEW_METRICS
        ).await?;

        // Grant super_admin and tenant_admin cluster access
        self.authz_service.add_permission_for_role(
            roles::SUPER_ADMIN, tenant, &cluster_resource, actions::CLUSTER_ADMIN
        ).await?;
        self.authz_service.add_permission_for_role(
            roles::TENANT_ADMIN, tenant, &cluster_resource, actions::CLUSTER_ADMIN
        ).await?;

        info!("Cluster permissions initialized successfully for tenant: {}", tenant);
        Ok(())
    }

    /// Helper method to create auth context from user/tenant info
    pub fn create_auth_context(&self, user_id: String, tenant_id: Option<String>) -> AuthContext {
        let tenant = tenant_id.unwrap_or_else(|| self.default_tenant.clone());
        AuthContext::new(user_id, tenant)
    }

    /// Verify that a user has the minimum required role for cluster operations
    pub async fn verify_minimum_cluster_role(
        &self,
        auth_ctx: &AuthContext,
        required_role: &str,
    ) -> Result<bool> {
        debug!(
            "Verifying minimum cluster role: user={}, tenant={}, required_role={}",
            auth_ctx.user_id, auth_ctx.tenant_id, required_role
        );

        let user_roles = self.authz_service
            .get_roles_for_user_in_tenant(&auth_ctx.user_id, &auth_ctx.tenant_id)
            .await?;

        // Check if user has the required role or a higher-level role
        let has_role = user_roles.contains(&required_role.to_string()) ||
            user_roles.contains(&roles::CLUSTER_ADMIN.to_string()) ||
            user_roles.contains(&roles::SUPER_ADMIN.to_string()) ||
            user_roles.contains(&roles::TENANT_ADMIN.to_string());

        debug!(
            "Role verification result: user={}, tenant={}, required_role={}, has_role={}, user_roles={:?}",
            auth_ctx.user_id, auth_ctx.tenant_id, required_role, has_role, user_roles
        );

        Ok(has_role)
    }
}

/// Authorization wrapper for Raft operations
#[derive(Clone)]
pub struct AuthorizedRaftOperation {
    /// The authorization context
    pub auth_ctx: AuthContext,
    /// The authorization result
    pub permission_result: PermissionResult,
}

impl AuthorizedRaftOperation {
    /// Create a new authorized operation
    pub fn new(auth_ctx: AuthContext, permission_result: PermissionResult) -> Self {
        Self {
            auth_ctx,
            permission_result,
        }
    }

    /// Check if the operation is authorized
    pub fn is_authorized(&self) -> bool {
        self.permission_result.allowed
    }

    /// Get authorization error if not allowed
    pub fn authorization_error(&self) -> Option<ConfluxError> {
        if !self.permission_result.allowed {
            Some(ConfluxError::AuthError(format!(
                "Access denied: user '{}' cannot perform '{}' on resource '{}' in tenant '{}'",
                self.permission_result.user_id,
                self.permission_result.action,
                self.permission_result.resource,
                self.permission_result.tenant_id
            )))
        } else {
            None
        }
    }

    /// Ensure the operation is authorized, returning an error if not
    pub fn ensure_authorized(&self) -> Result<()> {
        if let Some(error) = self.authorization_error() {
            Err(error)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test] 
    async fn test_auth_context_creation() {
        let service = create_test_service().await;
        let ctx = service.create_auth_context("user1".to_string(), Some("tenant1".to_string()));
        
        assert_eq!(ctx.user_id, "user1");
        assert_eq!(ctx.tenant_id, "tenant1");
    }

    #[tokio::test]
    async fn test_authorized_operation() {
        let auth_ctx = AuthContext::new("user1".to_string(), "tenant1".to_string());
        let permission_result = PermissionResult::allowed(
            "user1".to_string(),
            "tenant1".to_string(),
            "/cluster".to_string(),
            "read".to_string(),
        );
        
        let operation = AuthorizedRaftOperation::new(auth_ctx, permission_result);
        assert!(operation.is_authorized());
        assert!(operation.authorization_error().is_none());
        assert!(operation.ensure_authorized().is_ok());
    }

    #[tokio::test]
    async fn test_unauthorized_operation() {
        let auth_ctx = AuthContext::new("user1".to_string(), "tenant1".to_string());
        let permission_result = PermissionResult::denied(
            "user1".to_string(),
            "tenant1".to_string(),
            "/cluster".to_string(),
            "admin".to_string(),
        );
        
        let operation = AuthorizedRaftOperation::new(auth_ctx, permission_result);
        assert!(!operation.is_authorized());
        assert!(operation.authorization_error().is_some());
        assert!(operation.ensure_authorized().is_err());
    }

    async fn create_test_service() -> RaftAuthzService {
        // Create a mock authz service for testing
        // In a real test, this would use a test database
        let authz_service = Arc::new(
            AuthzService::new("postgresql://test:test@localhost/test")
                .await
                .unwrap_or_else(|_| {
                    // Return a mock service if database is not available
                    panic!("Test database not available")
                })
        );
        
        RaftAuthzService::new(authz_service, "default".to_string())
    }
}