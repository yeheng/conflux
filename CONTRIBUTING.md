# Rust 代码规范

## 1. 代码组织

- [x] 按功能划分模块，使用`mod.rs`明确导出关系
- [x] 避免跨模块循环依赖
- [x] 公共API使用`pub`显式标注
- [x] 私有模块使用`pub(crate)`控制可见性
- [x] 模块深度 ≤ 3 层（避免过度嵌套）
- [x] 单个文件 ≤ 200 行（超限时拆分子模块）
- [x] 禁止 mod.rs 超过 200 行

## 2. 命名规范

- [x] 结构体/枚举使用`UpperCamelCase`
- [x] 函数/方法使用`snake_case`
- [x] 常量使用`SCREAMING_SNAKE_CASE`
- [x] 类型参数使用单字母`T`, `U`, `V`等

## 3. 注释与文档

- [x] 公共API必须包含`///`文档注释
- [x] 使用`# Examples`标注示例代码，并说明输入输出
- [x] 使用`// FIXME:`标记待修复点
- [x] 模块级注释添加`//!`
- [x] 对核心逻辑进行说明，避免重复
- [x] 对入参出参进行详细描述，并且把边界条件和约束要说明，并说明输入输出
- [x] 复杂逻辑添加`// TODO:`标记待完善点
- [x] 模块级注释说明设计意图
- [x] 使用`cargo doc`生成文档

## 4. 错误处理

```rust
// ✅ 使用 thiserror + anyhow 组合
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error")]
    ParseFailed,
}

fn process() -> anyhow::Result<()> {
    let data = std::fs::read("config.toml")?; // 自动转换错误
    // ...
}
```

- [x] 库使用 thiserror 定义结构化错误
- [x] 二进制程序使用 anyhow 进行上下文包装
- [x] 禁止 unwrap()/expect() 生产环境代码
- [x] 错误必须携带上下文：context("Failed to read config")?
- [x] 错误类型必须实现`Error` trait

## 5. 测试规范

```rust
/// 计算两个数的和
///
/// # Examples
/// ```
/// assert_eq!(add(2, 3), 5);
/// ```
#[inline]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_add_basic() {
        assert_eq!(add(2, 3), 5);
    }

    proptest! {
        #[test]
        fn test_add_property(a in -100..100, b in -100..100) {
            assert_eq!(add(a, b), a + b);
        }
    }
}
```

- [x] 单元测试与代码分离，为源代码文件名后面+_test.rs，使用`#[cfg(test)]`
- [x] 集成测试放在`tests/`目录
- [x] 测试覆盖率必须 ≥ 80%
- [x] 测试用例覆盖所有`Result`分支
- [x] 使用`assert_matches!`验证错误类型
- [x] 使用`proptest`进行属性测试
- [x] 异步测试使用`#[tokio::test]`
- [x] 数据库测试使用事务回滚保证隔离

## 6. 性能优化

- [x] 避免不必要的`clone()`调用
- [x] 使用`Cow`处理借用/拥有数据
- [x] 优先使用迭代器而非显式循环
- [x] 使用`#![deny(clippy::perf)]`启用性能检查

## 7. 安全实践

- [x] 使用`#![deny(unsafe_code)]`限制不安全代码
- [x] 必须使用`unsafe`时单独模块隔离
- [x] 所有`unsafe`块需要详细注释说明
- [x] 使用`#![deny(clippy::security)]`启用安全检查

## 8. 依赖管理

- [x] 使用`cargo clippy`检查依赖安全
- [x] 依赖版本使用`^`限定最小版本
- [x] 开发依赖使用`dev-dependencies`
- [x] 使用`cargo audit`定期检查漏洞

## 9. 格式化标准

- [x] 使用`rustfmt`格式化代码
- [x] 配置`rustfmt.toml`统一格式
- [x] 所有代码提交前执行格式化
- [x] 使用`cargo fmt --all`批量格式化

## 10. 版本控制

- [x] 公共API变更遵循语义化版本
- [x] 使用`cargo clippy`检查API变更
- [x] 重大变更添加`#[deprecated]`标注
- [x] 使用`cargo doc`生成文档变更记录

## 11. 开发环境设置

- [x] 安装 Rust 1.70+ 和必要工具链
- [x] 配置 PostgreSQL 13+ 数据库
- [x] 设置环境变量 `DATABASE_URL=postgres://postgres:postgres@localhost:5432/conflux_test`
- [x] 运行数据库迁移 `sqlx migrate run`
- [x] 使用 `cargo test` 验证环境配置
- [x] 配置 IDE 支持 (推荐 rust-analyzer)

```bash
# 环境设置示例
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/conflux_test"
export RUST_LOG="info,conflux=debug"
export CONFLUX_NODE_ID=1

# 启动开发数据库
docker run -d --name postgres-dev \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=conflux_test \
  -p 5432:5432 postgres:15

# 运行迁移
sqlx migrate run

# 验证环境
cargo test --all-features
```

## 12. Git 工作流程

- [x] 分支命名：`feature/功能名`、`bugfix/问题描述`、`hotfix/紧急修复`
- [x] 提交格式：`type(scope): description` (遵循 Conventional Commits)
- [x] PR 标题遵循 Conventional Commits 规范
- [x] 每个 PR 必须通过所有 CI 检查
- [x] 至少一个代码审查者批准后方可合并
- [x] 合并前必须 rebase 到最新 main 分支

```bash
# 提交消息示例
feat(auth): add multi-tenant RBAC support
fix(raft): resolve leader election timeout issue
docs(api): update REST API documentation
test(storage): add integration tests for RocksDB
```

## 13. 异步编程规范

- [x] 优先使用 `async/await` 而非手动 Future 构造
- [x] 使用 `tokio::spawn` 处理并发任务
- [x] 异步函数返回 `Result<T, E>` 时使用 `?` 操作符
- [x] 避免在异步上下文中使用阻塞操作
- [x] 使用 `#[tokio::test]` 编写异步测试
- [x] 长时间运行的任务使用 `tokio::select!` 处理取消
- [x] 使用 `Arc<Mutex<T>>` 或 `Arc<RwLock<T>>` 共享状态

```rust
// ✅ 正确的异步错误处理
async fn process_config() -> anyhow::Result<()> {
    let data = fetch_data().await?;
    let result = transform_data(data).await?;
    store_result(result).await?;
    Ok(())
}

// ✅ 正确的并发处理
async fn process_multiple() -> anyhow::Result<Vec<String>> {
    let futures = items.into_iter().map(|item| {
        tokio::spawn(async move { process_item(item).await })
    });

    let results = futures::future::try_join_all(futures).await?;
    Ok(results.into_iter().collect::<Result<Vec<_>, _>>()?)
}
```

## 14. 数据库开发规范

- [x] 所有数据库变更通过 SQLx 迁移文件管理
- [x] 迁移文件命名：`YYYYMMDD_HHMMSS_描述.sql`
- [x] 禁止修改已应用的迁移文件
- [x] 使用 `sqlx::query!` 宏进行编译时 SQL 检查
- [x] 数据库连接使用连接池管理
- [x] 事务操作必须有明确的错误处理

```sql
-- 迁移文件示例: 20241211_120000_add_tenant_quotas.sql
CREATE TABLE tenant_quotas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    max_configs INTEGER NOT NULL DEFAULT 1000,
    max_versions_per_config INTEGER NOT NULL DEFAULT 100,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tenant_quotas_tenant_id ON tenant_quotas(tenant_id);
```

## 15. 日志和监控规范

- [x] 使用 `tracing` 而非 `log` 宏记录日志
- [x] 为关键函数添加 `#[instrument]` 属性
- [x] 错误日志必须包含足够的上下文信息
- [x] 使用结构化日志格式 (JSON)
- [x] 为业务指标添加 Prometheus 埋点
- [x] 关键操作添加 span 追踪

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(self), fields(tenant_id = %request.tenant_id))]
async fn create_config(&self, request: CreateConfigRequest) -> Result<Config> {
    info!("Creating new config");

    let config = self.store.create_config(request).await
        .map_err(|e| {
            error!("Failed to create config: {}", e);
            e
        })?;

    // 记录业务指标
    metrics::counter!("configs_created_total", 1, "tenant" => request.tenant_id);

    Ok(config)
}
```

## 16. 代码审查规范

- [x] 每个 PR 必须至少一个审查者批准
- [x] 审查者检查代码逻辑、性能、安全性
- [x] 审查者验证测试覆盖率和测试质量
- [x] 审查者确认文档更新的完整性
- [x] 使用 GitHub PR 模板确保信息完整
- [x] 重大变更需要架构师审查

### 审查清单

- [ ] 代码符合项目编码规范
- [ ] 测试覆盖率达到要求 (≥80%)
- [ ] 错误处理完整且合理
- [ ] 性能影响已评估
- [ ] 安全风险已考虑
- [ ] 文档已更新
- [ ] 向后兼容性已确认

## 17. CI/CD 流程说明

- [x] 所有 PR 必须通过 GitHub Actions 检查
- [x] CI 包括：格式检查、Clippy 检查、测试、安全审计
- [x] 测试失败时不允许合并
- [x] 使用 `cargo audit` 检查依赖漏洞
- [x] 主分支保护，禁止直接推送
- [x] 发布版本自动构建 Docker 镜像

### CI 检查项目

1. **格式检查**: `cargo fmt --all -- --check`
2. **代码检查**: `cargo clippy --all-targets --all-features -- -D warnings`
3. **测试运行**: `cargo test --all-features`
4. **安全审计**: `cargo audit`
5. **构建验证**: `cargo build --release --all-features`

附录：

1. [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
2. [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)
3. [Rustfmt Configuration](https://rust-lang.github.io/rustfmt/)
4. [Conventional Commits](https://www.conventionalcommits.org/)
5. [SQLx Documentation](https://docs.rs/sqlx/)
6. [Tracing Documentation](https://docs.rs/tracing/)
7. [Tokio Best Practices](https://tokio.rs/tokio/tutorial)
