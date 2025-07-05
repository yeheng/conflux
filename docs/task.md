### **Conflux 项目实施任务列表 (Task List)**

#### **Milestone 1: 核心原型 (Core Prototype - MVP)**

*目标：构建一个功能最小但端到端可用的系统，用于内部验证和早期测试。*

* **Epic: [CORE-1] 基础架构与项目设置**
  * [TASK-101] 初始化 Rust 项目 (`cargo new`)，设置 workspace。
  * [TASK-102] 引入 `tokio`, `serde`, `anyhow`, `thiserror`, `tracing` 等基础依赖。
  * [TASK-103] 搭建基本的 CI 流程 (Clippy, rustfmt, `cargo test`)。
  * [TASK-104] 设计并实现初步的配置文件加载逻辑。

* **Epic: [CORE-2] 共识与存储层**
  * [TASK-201] **(关键)** 设计并实现 `Store` 模块，使用 `rocksdb` 作为后端。
  * [TASK-202] **(关键)** 为 `Store` 实现 `openraft::RaftStorage` trait。
  * [TASK-203] 设计 `TypeConfig` (Raft 类型定义)。
  * [TASK-204] 实现 `RaftNetwork` trait，使用 `reqwest` 或 `tonic` 进行节点间通信。
  * [TASK-205] 封装 `openraft::Raft` 实例，创建 `RaftNode` 服务。
  * [TASK-206] 实现一个基本的、能处理 `client_write` 请求的接口。

* **Epic: [CORE-3] 状态机与核心业务逻辑**
  * [TASK-301] 定义核心数据结构: `Config`, `ConfigVersion`, `Release`。
  * [TASK-302] 定义核心 `RaftCommand`: `CreateVersion`, `UpdateReleaseRules`。
  * [TASK-303] **(关键)** 为 `Store` 实现 `RaftStateMachine` trait。
  * [TASK-304] 在 `apply` 方法中实现 `CreateVersion` 的逻辑。
  * [TASK-305] 在 `apply` 方法中实现 `UpdateReleaseRules` 的逻辑。
  * [TASK-306] 实现一个基本的 `get_published_config` 查询接口。

* **Epic: [CORE-4] 协议层 (HTTP)**
  * [TASK-401] 设计 `ProtocolPlugin` trait 和 `CoreAppHandle`。
  * [TASK-402] 实现一个基本的 HTTP 插件 (`axum`)。
  * [TASK-403] 实现 `POST /versions` API 端点，调用 `raft.client_write(CreateVersion)`。
  * [TASK-404] 实现 `PUT /releases` API 端点，调用 `raft.client_write(UpdateReleaseRules)`。
  * [TASK-405] 实现 `GET /fetch/config` API 端点，调用 `store.get_published_config`。

#### **Milestone 2: 提升可用性与安全性 (Usability & Security)**

*目标：增加关键的安全、认证和用户交互功能，使系统达到可被早期用户“友好使用”的程度。*

* **Epic: [SEC-1] 认证与授权 (Casbin)**
  * [TASK-501] 定义 Casbin `model.conf` (RBAC with tenants)。
  * [TASK-502] 集成 `casbin-sqlx-adapter`，并设置 `casbin_rule` 表。
  * [TASK-503] 实现 `AuthzService`，在应用启动时初始化 `Enforcer`。
  * [TASK-504] **(关键)** 实现 Axum 的授权中间件，调用 `authz_service.check`。
  * [TASK-505] 实现管理角色和权限的 API 端点。

* **Epic: [USER-1] 元数据与账户管理**
  * [TASK-601] 设计并创建 PostgreSQL 的 `tenants`, `users`, `roles` 等表。
  * [TASK-602] 实现 `MetadataService`。
  * [TASK-603] 实现用户 `login` API，包含密码哈希验证和 JWT 签发。
  * [TASK-604] 实现创建租户、用户的管理 API。

* **Epic: [USER-2] 客户端 SDK**
  * [TASK-701] 设计 SDK 的公共 API (`ConfluxClient`)。
  * [TASK-702] 实现内部缓存机制 (使用 `DashMap` 和 `watch` channel)。
  * [TASK-703] 实现后台的 `Poller` 任务（全量拉取）。
  * [TASK-704] 实现基本的 `get_string`, `get_bool` 等方法。

* **Epic: [CORE-5] 订阅/通知服务**
  * [TASK-801] 实现 `WatchService` (使用 `DashMap` 和 `broadcast` channel)。
  * [TASK-802] 在 `State Machine` 的 `apply` 方法成功后调用 `watch_service.notify`。
  * [TASK-803] 在协议层（例如，gRPC 插件）实现 `Watch` RPC，调用 `watch_service.subscribe`。
  * [TASK-804] 在客户端 SDK 中实现后台 `Watcher` 任务，连接 gRPC stream。

#### **Milestone 3: 生产环境就绪 (Production Readiness)**

*目标：增加运维、监控、部署等生产环境必需的功能，使系统健壮、可靠。*

* **Epic: [OPS-1] 集群运维与管理**
  * [TASK-901] 实现自动化的集群引导逻辑。
  * [TASK-902] 实现 `add_learner` 和 `change_membership` 的运维 API。
  * [TASK-903] 实现 `/_cluster/status` API，用于聚合集群状态。
  * [TASK-904] **(关键)** 实现备份（触发快照并上传）和恢复（从快照安装）的流程。

* **Epic: [OBS-1] 可观测性**
  * [TASK-1001] 集成 `metrics` 和 `prometheus` exporter，暴露 `/metrics` 端点。
  * [TASK-1002] 在代码中埋点，添加所有核心指标（HTTP, Raft, State Machine 等）。
  * [TASK-1003] 集成 `tracing`, `opentelemetry`，配置 OTLP 导出。
  * [TASK-1004] 在代码中添加 Trace Spans (`#[instrument]`)。
  * [TASK-1005] 配置结构化日志 (JSON)，并确保包含 `trace_id`。
  * [TASK-1006] 实现 `/health` 和 `/ready` 端点。

* **Epic: [SEC-2] 高级安全**
  * [TASK-1101] 为节点间通信配置和强制 mTLS。
  * [TASK-1102] 设计 `KmsProvider` trait。
  * [TASK-1103] 实现一个 KMS Provider (e.g., for AWS KMS)。
  * [TASK-1104] 在 `Publish` 流程中集成信封加密逻辑。
  * [TASK-1105] 在 SDK/API 中集成解密逻辑，包括 DEK 缓存。

* **Epic: [OPS-2] 打包与部署**
  * [TASK-1201] 编写一个多阶段的、优化的 `Dockerfile`。
  * [TASK-1202] **(关键)** 创建一个功能完备的 Helm Chart，包含 `StatefulSet`、`Service`、`ConfigMap` 等。
  * [TASK-1203] 在 `values.yaml` 中暴露所有关键配置项。
  * [TASK-1204] 建立一个 CI/CD 流程，用于自动构建镜像和发布 Helm Chart。

#### **Milestone 4: 生态系统与企业级功能 (Ecosystem & Enterprise Features)**

*目标：提供高级工作流和周边工具，提升开发者体验和企业采纳度。*

* **Epic: [DX-1] 高级工作流**
  * [TASK-1301] 设计并实现原子化的 `PublishingPlan` 和 `RaftCommand::Publish`。
  * [TASK-1302] 设计并实现“发布提案” (`ReleaseProposal`) 状态和相关 Raft 命令。
  * [TASK-1303] 实现 Webhook Service，用于在提案创建时发送通知。
  * [TASK-1304] 实现处理外部系统回调（`approve`/`reject`）的 API。

* **Epic: [DX-2] 配置即代码**
  * [TASK-1401] (Go 语言) 设计并开发 Terraform Provider for Conflux。
  * [TASK-1402] 为 Provider 实现 `conflux_config` 和 `conflux_release` 资源。
  * [TASK-1403] 编写 Terraform 的验收测试。
  * [TASK-1404] (可选) 设计并开发 GitOps Controller (Kubernetes Operator)。

* **Epic: [SYS-1] 系统健康与维护**
  * [TASK-1501] 在 `Config` 中增加 `retention_policy` 字段。
  * [TASK-1502] **(关键)** 在 Leader 节点上实现后台的 GC 任务。
  * [TASK-1503] 实现 `RaftCommand::PurgeVersions` 及其 `apply` 逻辑。
  * [TASK-1504] 设计并实现租户配额的原子化检查和用量计数器机制。

* **Epic: [DX-3] 命令行界面 (CLI)**
  * [TASK-1601] 使用 `clap` 设计 CLI 的命令结构。
  * [TASK-1602] 实现 `auth login` 和本地凭证管理。
  * [TASK-1603] 基于客户端 SDK 实现 `config` 和 `release` 子命令。
  * [TASK-1604] 实现 `admin` 子命令。

* **Epic: [DOCS-1] 文档与社区**
  * [TASK-1701] 选择并搭建文档网站（如 Docusaurus）。
  * [TASK-1702] 编写快速开始和核心概念文档。
  * [TASK-1703] 自动生成 API 和 CLI 参考手册。
  * [TASK-1704] 编写运维和贡献者指南。

---

这个任务列表提供了一个从 0 到 1 再到 N 的清晰路径。每个 Epic 都是一个大的功能块，每个 Task 都是一个具体的、可交付的开发任务。建议在实际的项目管理中，为每个 Task 估算工作量（例如，用故事点或小时），并分配给相应的开发者。
