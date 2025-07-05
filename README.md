# Conflux - 分布式配置中心

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/conflux-rs/conflux)

> 🚀 **高性能、分布式、安全的配置中心** - 基于 Rust 和 Raft 共识算法构建

## 📖 项目概述

Conflux 是一个现代化的分布式配置中心，旨在为微服务架构提供高可用、高性能的配置管理解决方案。系统采用 Rust 语言开发，基于 Raft 共识算法实现分布式一致性，支持多种协议访问和丰富的企业级功能。

### ✨ 核心特性

- 🔄 **分布式架构**: 基于 Raft 共识算法，无单点故障
- 🚀 **高性能**: 读取 5000 QPS，写入 2000 QPS，延迟 < 100ms
- 🔐 **安全可靠**: 端到端加密、RBAC 权限控制、审计日志
- 🔌 **多协议支持**: HTTP REST、gRPC、WebSocket、GraphQL
- 📝 **多格式配置**: JSON、YAML、TOML、XML、INI、Properties 等
- 🎯 **高级发布**: 蓝绿发布、灰度发布、指定节点发布
- 🏢 **多租户**: 租户 → 应用 → 环境 的层级管理
- 🔧 **插件化**: 基于 Rust traits 的可扩展架构

## 🏗️ 系统架构

```mermaid
graph TD
    subgraph "客户端层 (Clients)"
        CLI[CLI 工具]
        SDK[客户端 SDK]
        UI[Web 管理界面]
        APP[应用程序]
    end

    subgraph "负载均衡层"
        LB[负载均衡器<br/>Nginx/HAProxy]
    end

    subgraph "Conflux 集群 (Raft Cluster)"
        subgraph "Node 1 (Leader)"
            N1_API[协议层<br/>HTTP/gRPC/WS]
            N1_BIZ[业务逻辑层<br/>认证/授权/租户隔离]
            N1_RAFT[共识层<br/>OpenRaft]
            N1_SM[状态机<br/>配置存储]
            N1_STORE[存储层<br/>RocksDB]
        end
        
        subgraph "Node 2 (Follower)"
            N2_API[协议层]
            N2_RAFT[共识层]
            N2_SM[状态机]
            N2_STORE[存储层]
        end
        
        subgraph "Node 3 (Follower)"
            N3_API[协议层]
            N3_RAFT[共识层]
            N3_SM[状态机]
            N3_STORE[存储层]
        end
    end

    subgraph "外部依赖"
        PG[(PostgreSQL<br/>元数据库)]
        KMS[密钥管理服务<br/>AWS KMS/Vault]
        MONITOR[监控系统<br/>Prometheus/Grafana]
    end

    CLI --> LB
    SDK --> LB
    UI --> LB
    APP --> LB
    
    LB --> N1_API
    LB --> N2_API
    LB --> N3_API
    
    N1_API --> N1_BIZ --> N1_RAFT --> N1_SM --> N1_STORE
    N2_API --> N2_RAFT --> N2_SM --> N2_STORE
    N3_API --> N3_RAFT --> N3_SM --> N3_STORE
    
    N1_RAFT -.->|Raft 复制| N2_RAFT
    N2_RAFT -.->|Raft 复制| N3_RAFT
    N3_RAFT -.->|Raft 复制| N1_RAFT
    
    N1_BIZ --> PG
    N1_BIZ --> KMS
    N1_API --> MONITOR
```

## 📚 文档索引

### 🎯 快速开始
- [安装指南](docs/installation.md) - 快速部署和配置
- [快速开始](docs/quickstart.md) - 5分钟上手指南
- [基本概念](docs/concepts.md) - 核心概念和术语

### 🏛️ 架构设计
- [📋 项目概览](docs/overview.md) - 系统整体架构和组件说明
- [🏗️ 高层设计](docs/high-level-design.md) - 详细的技术架构方案
- [📊 数据模型](docs/data-model.md) - 核心数据结构和关系

### 🔧 核心组件
- [⚡ 核心共识层](docs/Core%20Consensus%20Layer.md) - Raft 共识算法实现
- [🗄️ 状态机](docs/State%20Machine.md) - 配置状态管理
- [💾 存储层](docs/Storage.md) - RocksDB 存储设计
- [🗃️ 元数据库](docs/Metadata%20DB.md) - PostgreSQL 元数据管理
- [🔌 协议层](docs/Protocol%20Layer.md) - 多协议支持架构

### 🚀 功能特性
- [📝 配置管理](docs/Config%20Managment.md) - 配置的 CRUD 操作
- [📋 配置即代码](docs/Config-as-Code.md) - GitOps 和 Terraform 集成
- [🔄 版本控制](docs/versioning.md) - 配置版本管理
- [🎯 发布策略](docs/deployment-strategies.md) - 蓝绿/灰度发布
- [👁️ 监听服务](docs/Watch%20Service.md) - 实时配置变更通知
- [🔐 安全加密](docs/Security%20&%20Encryption.md) - 端到端安全方案

### 🔐 安全与权限
- [🛡️ RBAC 权限](docs/RBAC.md) - 基于角色的访问控制
- [🏢 多租户](docs/Multi-tenancy%20&%20Quota%20Management.md) - 租户隔离和配额管理
- [✅ 审批流程](docs/Approval%20Workflow.md) - 配置变更审批

### 🛠️ 开发与运维
- [💻 客户端 SDK](docs/Client%20SDK.md) - 多语言 SDK 使用指南
- [⌨️ 命令行工具](docs/CLI.md) - CLI 工具使用手册
- [🚀 部署分发](docs/Packaging,%20Deployment%20&%20Distribution.md) - Docker/K8s 部署
- [⚙️ 集群运维](docs/Cluster%20Operations%20&%20Management.md) - 集群管理和维护
- [📊 可观测性](docs/Observability.md) - 监控、日志、追踪
- [🗑️ 数据生命周期](docs/Data%20Lifecycle%20&%20Garbage%20Collection.md) - 数据清理和归档

### 📋 项目管理
- [📝 任务清单](docs/task.md) - 开发里程碑和任务分解
- [🎯 技术规范](rust.md) - Rust 开发规范和原则

## 📊 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 读取 QPS | 5,000+ | 配置拉取请求/秒 |
| 写入 QPS | 2,000+ | 配置更新请求/秒 |
| 查询延迟 | < 100ms | P99 响应时间 |
| 变更通知延迟 | < 100ms | 配置变更推送延迟 |
| 并发连接数 | 5,000+ | Watch/Subscribe 连接 |
| 可用性 | 99.9% | 服务可用性保证 |

## 🛠️ 技术栈

### 核心技术

- **语言**: Rust 2024 Edition
- **异步运行时**: Tokio
- **Web 框架**: Axum
- **共识算法**: OpenRaft
- **存储引擎**: RocksDB
- **元数据库**: PostgreSQL
- **序列化**: Serde

### 监控与运维

- **指标收集**: Prometheus
- **链路追踪**: OpenTelemetry
- **容器化**: Docker
- **编排**: Kubernetes
- **包管理**: Helm

### 安全组件

- **TLS**: Rustls
- **加密**: Ring/Age
- **密钥管理**: 外部 KMS (AWS KMS, Vault)
- **认证**: JWT
- **授权**: RBAC

## 🚀 快速开始

### 前置要求

- Rust 1.70+
- PostgreSQL 13+
- Docker & Docker Compose (可选)
- Kubernetes (生产环境)

### 本地开发

```bash
# 克隆项目
git clone https://github.com/conflux-rs/conflux.git
cd conflux

# 安装依赖
cargo build

# 启动开发环境
docker-compose up -d postgres

# 运行测试
cargo test

# 启动服务
cargo run
```

### Docker 部署

```bash
# 构建镜像
docker build -t conflux:latest .

# 使用 Docker Compose 启动
docker-compose up -d
```

### Kubernetes 部署

```bash
# 使用 Helm 部署
helm repo add conflux https://charts.conflux.rs
helm install conflux conflux/conflux
```

## 🔧 配置示例

### 基本配置

```yaml
# conflux.yaml
server:
  host: "0.0.0.0"
  port: 8080
  
cluster:
  node_id: 1
  peers:
    - "node-1:9090"
    - "node-2:9090"
    - "node-3:9090"

database:
  url: "postgresql://user:pass@localhost/conflux"
  
storage:
  data_dir: "/var/lib/conflux"
```

### 客户端使用

```rust
use conflux_sdk::ConfluxClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ConfluxClient::new("http://localhost:8080")
        .with_token("your-jwt-token")
        .build()
        .await?;

    // 获取配置
    let config: String = client
        .get_config("my-tenant", "my-app", "production", "database.toml")
        .await?;

    // 监听配置变更
    let mut watcher = client
        .watch_config("my-tenant", "my-app", "production", "database.toml")
        .await?;

    while let Some(change) = watcher.next().await {
        println!("配置已更新: {:?}", change);
    }

    Ok(())
}
```

### REST API 示例

```bash
# 获取配置
curl -H "Authorization: Bearer <token>" \
  http://localhost:8080/api/v1/fetch/config/my-tenant/my-app/production/database.toml

# 创建新配置版本
curl -X POST \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/toml" \
  -d @database.toml \
  http://localhost:8080/api/v1/mgt/configs/123/versions

# 更新发布规则（灰度发布）
curl -X PUT \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "releases": [
      {
        "labels": {"canary": "true"},
        "version_id": 5,
        "priority": 100
      },
      {
        "labels": {},
        "version_id": 4,
        "priority": 1
      }
    ]
  }' \
  http://localhost:8080/api/v1/mgt/configs/123/releases
```

## 🎯 使用场景

### 微服务配置管理

- 统一管理数百个微服务的配置文件
- 支持不同环境（开发/测试/生产）的配置隔离
- 实时推送配置变更，无需重启服务

### 功能开关管理

- 动态控制功能的开启和关闭
- 支持基于用户标签的精细化控制
- A/B 测试和灰度发布支持

### 多租户 SaaS 平台

- 为不同租户提供独立的配置空间
- 细粒度的权限控制和审计
- 支持租户级别的配额管理

### DevOps 自动化

- 与 CI/CD 流水线集成
- 支持 GitOps 工作流
- 配置变更的审批和回滚机制

## 🤝 贡献指南

我们欢迎社区贡献！请查看 [贡献指南](CONTRIBUTING.md) 了解如何参与项目开发。

### 开发流程

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [OpenRaft](https://github.com/datafuselabs/openraft) - Raft 共识算法实现
- [Tokio](https://tokio.rs/) - 异步运行时
- [Axum](https://github.com/tokio-rs/axum) - Web 框架
- [RocksDB](https://rocksdb.org/) - 存储引擎

---

**⭐ 如果这个项目对你有帮助，请给我们一个 Star！**

**📧 联系我们**: [team@conflux.rs](mailto:team@conflux.rs)

**🌐 官方网站**: [https://conflux.rs](https://conflux.rs)

**📖 在线文档**: [https://docs.conflux.rs](https://docs.conflux.rs)
