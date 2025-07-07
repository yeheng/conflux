# Conflux 认证与授权系统

基于 Casbin 的多租户 RBAC (基于角色的访问控制) 系统。

## 概述

Conflux 的认证授权系统提供了以下核心功能：

- **多租户支持**: 每个租户拥有独立的权限域
- **基于角色的访问控制**: 支持角色继承和权限分配
- **细粒度权限控制**: 支持资源路径模式匹配
- **动态权限管理**: 运行时添加/删除权限和角色
- **高性能**: 基于 Casbin 的高效权限检查引擎

## 架构组件

### 1. AuthzService

核心授权服务，封装了 Casbin Enforcer：

```rust
use conflux::auth::AuthzService;

// 初始化服务
let authz_service = AuthzService::new("postgresql://...").await?;

// 检查权限
let allowed = authz_service
    .check("user_id", "tenant_id", "/resource/path", "action")
    .await?;
```

### 2. 中间件

Axum 中间件，自动进行权限检查：

```rust
use conflux::auth::authz_middleware;

let app = Router::new()
    .route("/api/protected", get(handler))
    .layer(from_fn_with_state(authz_service, authz_middleware));
```

### 3. 管理 API

REST API 端点用于管理权限和角色：

```rust
use conflux::auth::create_auth_routes;

let auth_routes = create_auth_routes(authz_service);
let app = Router::new().nest("/auth", auth_routes);
```

## 权限模型

### Casbin 模型配置

```ini
[request_definition]
r = sub, dom, obj, act

[policy_definition]
p = sub, dom, obj, act

[role_definition]
g = _, _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = g(r.sub, p.sub, r.dom) && r.dom == p.dom && keyMatch2(r.obj, p.obj) && r.act == p.act
```

### 权限格式

- **sub**: 角色名称
- **dom**: 租户 ID
- **obj**: 资源路径（支持通配符）
- **act**: 操作类型

### 角色分配格式

- **用户**: 用户 ID
- **角色**: 角色名称
- **域**: 租户 ID

## 预定义角色

```rust
use conflux::auth::roles;

// 超级管理员 - 拥有所有权限
roles::SUPER_ADMIN

// 租户管理员 - 管理特定租户
roles::TENANT_ADMIN

// 开发者 - 读写应用配置
roles::DEVELOPER

// 查看者 - 只读权限
roles::VIEWER
```

## 预定义操作

```rust
use conflux::auth::actions;

actions::READ    // 读取
actions::WRITE   // 写入
actions::DELETE  // 删除
actions::ADMIN   // 管理
```

## 资源路径模式

使用 `ResourcePath` 构建器创建标准化的资源路径：

```rust
use conflux::auth::ResourcePath;

// 配置文件路径
let path = ResourcePath::config("tenant1", "app1", "prod", "db.toml");
// 结果: "/tenants/tenant1/apps/app1/envs/prod/configs/db.toml"

// 应用路径
let path = ResourcePath::app("tenant1", "app1");
// 结果: "/tenants/tenant1/apps/app1"

// 管理路径
let path = ResourcePath::admin("tenant1", "users");
// 结果: "/tenants/tenant1/admin/users"
```

## 使用示例

### 1. 基本权限检查

```rust
use conflux::auth::{AuthzService, actions};

let authz_service = AuthzService::new(database_url).await?;

// 检查用户是否有读取权限
let allowed = authz_service
    .check("alice", "tenant1", "/tenants/tenant1/apps/myapp", actions::READ)
    .await?;

if allowed {
    println!("用户有权限访问资源");
} else {
    println!("用户没有权限访问资源");
}
```

### 2. 角色和权限管理

```rust
// 为角色添加权限
authz_service
    .add_permission_for_role("developer", "tenant1", "/tenants/tenant1/apps/*", "read")
    .await?;

// 为用户分配角色
authz_service
    .assign_role_to_user("alice", "developer", "tenant1")
    .await?;

// 获取用户角色
let roles = authz_service
    .get_roles_for_user_in_tenant("alice", "tenant1")
    .await?;
```

### 3. 在 Axum 中使用

```rust
use axum::{routing::get, Router, middleware::from_fn_with_state};
use conflux::auth::{authz_middleware, AuthContext};

async fn protected_handler(
    Extension(auth_ctx): Extension<AuthContext>,
) -> String {
    format!("Hello, {}!", auth_ctx.user_id)
}

let app = Router::new()
    .route("/protected", get(protected_handler))
    .layer(from_fn_with_state(authz_service, authz_middleware))
    .with_state(app_state);
```

## 数据库设置

### 1. 创建数据库表

运行迁移脚本创建 `casbin_rule` 表：

```sql
-- 见 migrations/001_create_casbin_rule.sql
```

### 2. 初始数据

系统会自动插入一些初始的角色和权限数据。

## API 端点

### 权限检查

```
POST /_auth/check
Content-Type: application/json

{
  "user_id": "alice",
  "tenant": "tenant1",
  "resource": "/tenants/tenant1/apps/myapp",
  "action": "read"
}
```

### 角色管理

```
GET /tenants/{tenant}/users/{user_id}/roles
POST /tenants/{tenant}/users/{user_id}/roles
DELETE /tenants/{tenant}/users/{user_id}/roles/{role}
```

### 权限管理

```
POST /tenants/{tenant}/roles/{role}/permissions
DELETE /tenants/{tenant}/roles/{role}/permissions
```

### 策略重新加载

```
POST /_auth/reload
```

## 测试

运行单元测试：

```bash
cargo test --lib
```

运行演示程序（需要 PostgreSQL）：

```bash
# 启动 PostgreSQL
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=password postgres

# 运行演示
cargo run --example auth_demo
```

## 配置

在应用配置中设置数据库连接：

```toml
[database]
url = "postgresql://username:password@localhost:5432/conflux"
```

## 安全考虑

1. **认证**: 当前实现使用简化的 token 格式，生产环境应使用 JWT
2. **传输安全**: 确保使用 HTTPS
3. **数据库安全**: 使用强密码和网络隔离
4. **权限最小化**: 遵循最小权限原则分配角色

## 性能优化

1. **缓存**: Casbin 内置了策略缓存
2. **连接池**: 使用 SQLx 连接池
3. **索引**: 数据库表已创建适当的索引
4. **批量操作**: 支持批量添加/删除权限

## 故障排除

### 常见问题

1. **数据库连接失败**: 检查连接字符串和数据库状态
2. **权限检查失败**: 确认角色和权限配置正确
3. **中间件不工作**: 检查中间件注册顺序

### 调试

启用详细日志：

```rust
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```
