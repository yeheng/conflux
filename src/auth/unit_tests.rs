use super::*;

/// 测试用的数据库设置
/// 
/// 注意：这些测试需要一个运行中的PostgreSQL实例
/// 可以通过docker运行：docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=password postgres

#[cfg(test)]
mod integration_tests {
    use super::*;
    use sqlx::PgPool;

    // 这个测试需要真实的数据库连接，所以我们先跳过
    #[tokio::test]
    #[ignore = "需要PostgreSQL数据库"]
    async fn test_authz_service_creation() {
        // 这里需要一个真实的数据库连接字符串
        let database_url = "postgresql://postgres:password@localhost:5432/conflux_test";
        
        let pool = PgPool::connect(database_url).await.unwrap();
        
        // 创建casbin_rule表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS casbin_rule (
                id SERIAL PRIMARY KEY,
                ptype VARCHAR(100) NOT NULL,
                v0 VARCHAR(100) NOT NULL,
                v1 VARCHAR(100) NOT NULL,
                v2 VARCHAR(100) NOT NULL,
                v3 VARCHAR(100) NOT NULL,
                v4 VARCHAR(100) NOT NULL DEFAULT '',
                v5 VARCHAR(100) NOT NULL DEFAULT ''
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let authz_service = AuthzService::new(database_url).await.unwrap();
        
        // 测试权限检查
        let result = authz_service
            .check("user1", "tenant1", "/test", "read")
            .await
            .unwrap();
        
        // 初始状态应该没有权限
        assert!(!result);
    }

    #[tokio::test]
    #[ignore = "需要PostgreSQL数据库"]
    async fn test_permission_management() {
        let database_url = "postgresql://postgres:password@localhost:5432/conflux_test";
        let pool = PgPool::connect(database_url).await.unwrap();
        
        // 清理测试数据
        sqlx::query("DELETE FROM casbin_rule").execute(&pool).await.unwrap();
        
        let authz_service = AuthzService::new(database_url).await.unwrap();
        
        // 添加权限
        let result = authz_service
            .add_permission_for_role("admin", "tenant1", "/test/*", "read")
            .await
            .unwrap();
        assert!(result);
        
        // 分配角色
        let result = authz_service
            .assign_role_to_user("user1", "admin", "tenant1")
            .await
            .unwrap();
        assert!(result);
        
        // 检查权限
        let result = authz_service
            .check("user1", "tenant1", "/test/config", "read")
            .await
            .unwrap();
        assert!(result);
        
        // 检查没有权限的操作
        let result = authz_service
            .check("user1", "tenant1", "/test/config", "write")
            .await
            .unwrap();
        assert!(!result);
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_auth_context() {
        let ctx = AuthContext::new("user1".to_string(), "tenant1".to_string());
        assert_eq!(ctx.user_id, "user1");
        assert_eq!(ctx.tenant_id, "tenant1");
        assert!(ctx.roles.is_none());
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
        assert_eq!(allowed.user_id, "user1");
        assert_eq!(allowed.tenant_id, "tenant1");
        assert_eq!(allowed.resource, "/resource");
        assert_eq!(allowed.action, "read");
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

        assert_eq!(
            ResourcePath::admin("tenant1", "users"),
            "/tenants/tenant1/admin/users"
        );
    }

    #[test]
    fn test_constants() {
        assert_eq!(actions::READ, "read");
        assert_eq!(actions::WRITE, "write");
        assert_eq!(actions::DELETE, "delete");
        assert_eq!(actions::ADMIN, "admin");

        assert_eq!(roles::SUPER_ADMIN, "super_admin");
        assert_eq!(roles::TENANT_ADMIN, "tenant_admin");
        assert_eq!(roles::DEVELOPER, "developer");
        assert_eq!(roles::VIEWER, "viewer");
    }
}
