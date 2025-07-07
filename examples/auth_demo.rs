use conflux::auth::{AuthzService, actions, roles, ResourcePath};
use std::sync::Arc;
use tracing_subscriber::fmt::init;

/// 演示认证授权系统的基本功能
/// 
/// 这个示例展示了如何：
/// 1. 初始化AuthzService
/// 2. 设置角色和权限
/// 3. 分配用户角色
/// 4. 检查权限
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    init();
    
    println!("🚀 Conflux 认证授权系统演示");
    println!("================================");
    
    // 注意：这需要一个运行中的PostgreSQL数据库
    // 可以通过以下命令启动：
    // docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=password postgres
    let database_url = "postgresql://postgres:password@localhost:5432/conflux_demo";
    
    println!("📊 连接数据库: {}", database_url);
    
    // 初始化AuthzService
    let authz_service = match AuthzService::new(database_url).await {
        Ok(service) => {
            println!("✅ AuthzService 初始化成功");
            Arc::new(service)
        }
        Err(e) => {
            println!("❌ AuthzService 初始化失败: {}", e);
            println!("💡 请确保PostgreSQL数据库正在运行并且连接字符串正确");
            println!("   可以使用以下命令启动PostgreSQL:");
            println!("   docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=password postgres");
            return Err(e.into());
        }
    };
    
    println!("\n🔧 设置权限策略");
    println!("================");
    
    // 设置租户管理员权限
    authz_service
        .add_permission_for_role(
            roles::TENANT_ADMIN,
            "demo_tenant",
            "/tenants/demo_tenant/*",
            actions::ADMIN,
        )
        .await?;
    println!("✅ 租户管理员权限设置完成");
    
    // 设置开发者权限
    authz_service
        .add_permission_for_role(
            roles::DEVELOPER,
            "demo_tenant",
            "/tenants/demo_tenant/apps/*",
            actions::READ,
        )
        .await?;
    
    authz_service
        .add_permission_for_role(
            roles::DEVELOPER,
            "demo_tenant",
            "/tenants/demo_tenant/apps/*",
            actions::WRITE,
        )
        .await?;
    println!("✅ 开发者权限设置完成");
    
    // 设置查看者权限
    authz_service
        .add_permission_for_role(
            roles::VIEWER,
            "demo_tenant",
            "/tenants/demo_tenant/apps/*",
            actions::READ,
        )
        .await?;
    println!("✅ 查看者权限设置完成");
    
    println!("\n👥 分配用户角色");
    println!("================");
    
    // 分配用户角色
    authz_service
        .assign_role_to_user("alice", roles::TENANT_ADMIN, "demo_tenant")
        .await?;
    println!("✅ Alice 被分配为租户管理员");
    
    authz_service
        .assign_role_to_user("bob", roles::DEVELOPER, "demo_tenant")
        .await?;
    println!("✅ Bob 被分配为开发者");
    
    authz_service
        .assign_role_to_user("charlie", roles::VIEWER, "demo_tenant")
        .await?;
    println!("✅ Charlie 被分配为查看者");
    
    println!("\n🔍 权限检查测试");
    println!("================");
    
    // 测试各种权限检查
    let test_cases = vec![
        ("alice", "demo_tenant", "/tenants/demo_tenant/admin/users", actions::ADMIN, "管理员访问用户管理"),
        ("alice", "demo_tenant", "/tenants/demo_tenant/apps/myapp", actions::WRITE, "管理员写入应用配置"),
        ("bob", "demo_tenant", "/tenants/demo_tenant/apps/myapp/configs/db.toml", actions::READ, "开发者读取配置"),
        ("bob", "demo_tenant", "/tenants/demo_tenant/apps/myapp/configs/db.toml", actions::WRITE, "开发者写入配置"),
        ("bob", "demo_tenant", "/tenants/demo_tenant/admin/users", actions::ADMIN, "开发者访问用户管理（应该被拒绝）"),
        ("charlie", "demo_tenant", "/tenants/demo_tenant/apps/myapp/configs/db.toml", actions::READ, "查看者读取配置"),
        ("charlie", "demo_tenant", "/tenants/demo_tenant/apps/myapp/configs/db.toml", actions::WRITE, "查看者写入配置（应该被拒绝）"),
    ];
    
    for (user, tenant, resource, action, description) in test_cases {
        let allowed = authz_service.check(user, tenant, resource, action).await?;
        let status = if allowed { "✅ 允许" } else { "❌ 拒绝" };
        println!("{} {} - {} 在 {} 对 {} 执行 {}", 
                 status, user, description, tenant, resource, action);
    }
    
    println!("\n📋 用户角色查询");
    println!("================");
    
    let users = vec!["alice", "bob", "charlie"];
    for user in users {
        let roles = authz_service
            .get_roles_for_user_in_tenant(user, "demo_tenant")
            .await?;
        println!("👤 {} 的角色: {:?}", user, roles);
    }
    
    println!("\n🎯 资源路径构建器演示");
    println!("======================");
    
    let config_path = ResourcePath::config("demo_tenant", "myapp", "production", "database.toml");
    println!("📄 配置文件路径: {}", config_path);
    
    let app_path = ResourcePath::app("demo_tenant", "myapp");
    println!("📱 应用路径: {}", app_path);
    
    let admin_path = ResourcePath::admin("demo_tenant", "users");
    println!("⚙️  管理路径: {}", admin_path);
    
    println!("\n🎉 演示完成！");
    println!("==============");
    println!("认证授权系统已成功演示以下功能：");
    println!("• 基于角色的访问控制 (RBAC)");
    println!("• 多租户支持");
    println!("• 细粒度权限控制");
    println!("• 资源路径模式匹配");
    println!("• 动态权限检查");
    
    Ok(())
}
