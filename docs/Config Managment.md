### **核心模块详细设计：配置生命周期管理 (版本、发布与回滚)**

该模块负责管理一个配置项从创建、迭代、发布到生产，以及在出现问题时安全回滚的整个生命周期。它旨在提供一个清晰、可审计、低风险的变更管理流程。

#### **1. 接口设计 (API Design)**

我们将围绕“配置项 (`Config`)”这个资源来组织 API，提供管理其版本和发布状态的能力。这些是给**管理员和 CI/CD** 使用的管理平面 API。

| Endpoint | Method | Description |
| :--- | :--- | :--- |
| `/configs/{config_id}/versions` | `POST` | **创建新版本**: 上传新的配置内容，生成一个**未发布**的新版本。 |
| `/configs/{config_id}/versions` | `GET` | **列出历史版本**: 分页查看该配置的所有历史版本元数据。 |
| `/configs/{config_id}/versions/{version_id}` | `GET` | **获取特定版本**: 查看某个历史版本的详细信息和内容。 |
| `/configs/{config_id}/releases` | `PUT` | **发布/更新发布策略**: 定义哪些客户端获取哪个版本（核心发布操作）。 |
| `/configs/{config_id}/releases` | `GET` | **查看当前发布策略**: 查看当前的蓝绿/灰度规则。 |
| `/configs/{config_id}/rollback` | `POST` | **一键回滚**: 一个简化的快捷方式，用于快速将默认流量指回一个旧的稳定版本。 |

**设计理念:**

* **创建与发布分离**: `POST /versions` 只负责创建和校验一个新的、不可变的 `ConfigVersion`，它**不影响**任何线上流量。这是一个低风险操作。
* **发布是核心**: `PUT /releases` 是唯一一个会影响线上客户端配置的“高风险”操作。它将一个或多个已存在的版本“发布”给特定的客户端群体。
* **回滚是特殊的发布**: 回滚操作在内部只是 `PUT /releases` 的一个特例，它简化了 API 调用，使其更符合紧急情况下的心智模型。

---

#### **2. 出参入参设计 (Input/Output Parameter Design)**

##### **输入参数 (Inputs)**

1. **`POST /configs/{id}/versions` (创建新版本)**

    ```json
    {
      "content": "pool_size = 30", // base64 encoded or raw string
      "format": "toml",
      "description": "EMERGENCY: Increase pool size to 30 to handle traffic spike",
      "validate_only": false // 可选，如果为true，只做校验不创建
    }
    ```

2. **`PUT /configs/{id}/releases` (发布)**

    ```json
    {
      "rules": [
        // 规则1: 将新创建的 v3 版本发布给灰度用户
        {
          "labels": { "canary": "true" },
          "version_id": 3,
          "priority": 10
        },
        // 规则2: 其他所有用户继续使用稳定的 v2 版本
        {
          "labels": {}, // 空 labels map 代表默认规则
          "version_id": 2,
          "priority": 0
        }
      ],
      "comment": "Canary release of v3 for performance testing."
    }
    ```

3. **`POST /configs/{id}/rollback` (回滚)**

    ```json
    {
      "target_version_id": 1, // 要回滚到的稳定版本ID
      "comment": "Rollback to v1 due to high error rate in v2."
    }
    ```

##### **输出参数 (Outputs)**

1. **`GET /configs/{id}/versions` (列出版本)**

    ```json
    {
      "versions": [
        { "id": 3, "description": "...", "creator": "alice", "created_at": "..." },
        { "id": 2, "description": "...", "creator": "bob", "created_at": "..." }
      ],
      "pagination": { "next_cursor": "..." }
    }
    ```

---

#### **3. 数据模型设计 (Data Model Design)**

此模块的设计已经体现在我们之前的核心数据模型中，这里再次强调它们之间的关系：

* **`Config`**: 存储元数据，最重要的是 `releases: Vec<Release>` 字段，它定义了当前的发布策略。
* **`ConfigVersion`**: 存储不可变的内容快照。每个 `ConfigVersion` 都有一个唯一的 `version_id`。
* **`Release`**: `Config` 中的一个子结构，定义了“一组标签”到“一个 `version_id`”的映射。

**Raft 命令扩展:**

为了支持上述 API，我们需要以下 `RaftCommand` 变体：

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RaftCommand {
    // ... 其他命令 ...

    // 创建一个新版本
    CreateVersion {
        config_id: u64,
        content: Vec<u8>,
        format: ConfigFormat,
        description: String,
        // ... 其他元数据 ...
    },
    
    // 全量更新一个配置的发布规则集
    UpdateReleaseRules {
        config_id: u64,
        rules: Vec<Release>,
        comment: String,
    },
}
```

---

#### **4. 核心流程设计 (Core Flow Design)**

##### **a) 标准发布流程 (灰度发布一个新版本)**

这是一个两步工作流，由 CI/CD 系统执行。

```mermaid
sequenceDiagram
    participant CI/CD as CI/CD Pipeline
    participant API as Conflux API
    participant Raft as Raft Core
    participant SM as State Machine

    CI/CD->>API: 1. POST /configs/123/versions (content for v3)
    API->>API: Validate schema, etc.
    API->>Raft: Submit RaftCommand::CreateVersion
    Raft->>SM: apply(CreateVersion)
    SM->>SM: Create and store new ConfigVersion (id=3)
    SM-->>Raft-->>API: Ok, new_version_id = 3
    API-->>CI/CD: Return { "version_id": 3 }

    Note over CI/CD: Pipeline may run automated tests against v3<br>in a staging environment.

    CI/CD->>API: 2. PUT /configs/123/releases (rules pointing to v2 and v3)
    API->>Raft: Submit RaftCommand::UpdateReleaseRules
    Raft->>SM: apply(UpdateReleaseRules)
    SM->>SM: Overwrite the `releases` field in Config (id=123)
    SM-->>Raft-->>API: Ok
    API-->>CI/CD: 200 OK, Release successful
```

##### **b) 紧急回滚流程**

这是一个单步工作流，可由 SRE 手动触发。

```mermaid
sequenceDiagram
    participant SRE as Site Reliability Engineer
    participant API as Conflux API
    participant Raft as Raft Core
    participant SM as State Machine

    SRE->>API: POST /configs/123/rollback (target_version_id=1)
    
    Note over API: This is a simplified API. Internally, it<br>constructs a full release rule set.

    API->>API: Load current Config (id=123)
    API->>API: Create a new `rules` list, e.g., <br>[ { labels:{}, version_id:1, priority:0 } ]
    API->>Raft: Submit RaftCommand::UpdateReleaseRules
    Raft->>SM: apply(UpdateReleaseRules)
    SM->>SM: Overwrite the `releases` field
    SM-->>Raft-->>API: Ok
    API-->>SRE: 200 OK, Rollback initiated
```

---

#### **5. 关键逻辑详细说明 (Key Logic Details)**

##### **a) 为什么创建和发布要分离？**

这是为了实现**风险隔离**。

* **创建版本 (`CreateVersion`)** 是一个安全的写操作。它不会影响任何正在运行的应用。开发者可以随时向 Conflux 推送他们的新配置版本，而不用担心搞坏生产环境。这鼓励了频繁的集成。
* **发布 (`UpdateReleaseRules`)** 是一个有风险的决策操作。它改变了“谁看到什么”的规则。这个操作应该受到更严格的权限控制，并且应该在所有测试都通过后才执行。

这种分离使得权限管理更细粒度。例如，一个`developer`角色可以有权限`CreateVersion`，但只有`release-manager`角色才有权限`UpdateReleaseRules`。

##### **b) 回滚 API 的实现**

`rollback` API 的 handler 是一个“语法糖”。它的实现逻辑如下：

1. 接收到 `target_version_id`。
2. 加载对应的 `Config` 对象，以检查 `target_version_id` 是否是该配置的一个合法的历史版本。
3. 构造一个新的、**最简单**的 `Vec<Release>`，它只包含一条规则：一条 `priority` 为 0、`labels` 为空的默认规则，其 `version_id` 指向 `target_version_id`。
4. 将这个新的规则列表封装成 `RaftCommand::UpdateReleaseRules` 并提交。

这个实现方式保证了回滚操作会**清除所有**复杂的灰度或蓝绿规则，强制所有客户端都回退到指定的稳定版本，这正是在紧急情况下所期望的行为。

---

#### **6. 详细测试用例和测试方法 (Detailed Test Cases & Methods)**

##### **a) 单元测试**

* **`test_create_version_validation`**: 测试 `POST /versions` API 在内容不符合 schema 时返回 400。
* **`test_update_releases_invalid_version_id`**: 测试 `PUT /releases` 在引用一个不存在的 `version_id` 时返回 400。
* **`test_rollback_handler_logic`**: 单元测试 `rollback` API 的 handler，验证它能正确地构造出只包含一条默认规则的 `UpdateReleaseRules` 命令。

##### **b) 集成测试**

* **`test_full_release_workflow`**:
    1. 调用 `POST /versions` 创建 v1。
    2. 调用 `PUT /releases` 将 v1 发布为默认版本。
    3. 使用客户端 SDK 拉取配置，验证得到 v1 的内容。
    4. 调用 `POST /versions` 创建 v2。
    5. 调用 `PUT /releases`，将 v2 发布给 `canary:true` 的客户端，默认版本仍为 v1。
    6. 使用带 `canary:true` 标签的 SDK 拉取，验证得到 v2。
    7. 使用不带标签的 SDK 拉取，验证仍然得到 v1。
* **`test_rollback_workflow`**:
    1. 延续上述测试的状态。
    2. 调用 `POST /rollback`，目标为 v1。
    3. 使用带 `canary:true` 标签的 SDK 拉取，验证现在也得到了 v1 的内容（因为灰度规则被清除了）。

---

#### **7. 设计依赖 (Dependencies)**

* **Raft 状态机**: 负责持久化 `Config` 和 `ConfigVersion`，并原子地应用 `RaftCommand`。
* **认证与授权模块 (Casbin)**: 保护这些管理 API，确保只有授权用户才能执行创建、发布和回滚操作。
* **客户端 SDK**: 消费此模块产生的结果，根据发布规则拉取正确的配置版本。

---

#### **8. 已知存在问题 (Known Issues)**

1. **发布策略的复杂性**: 手动编写 `PUT /releases` 的 JSON body 仍然很容易出错，尤其是当规则很多时。
2. **“回滚”的二义性**: 当前的 `rollback` API 会清除所有灰度规则。在某些非紧急场景下，用户可能只想回滚“默认版本”，而保持灰度规则不变。
3. **缺乏发布审批流**: 当前设计中，只要有权限，就可以直接发布。在大型企业中，发布通常需要多方审批。

---

#### **9. 可迭代 Enhancement (Potential Enhancements)**

1. **发布模板 (Release Templates)**:
    * 提供更高层次的 API，例如 `POST /releases/canary`，其 body 更简单：`{"canary_version_id": 3, "stable_version_id": 2, "canary_percent": 10}`。
    * 由 Conflux 服务端根据这个模板来生成底层的 `Release` 规则集。这极大地降低了客户端的复杂性。
2. **更智能的回滚**:
    * 为 `POST /rollback` API 增加一个可选参数 `mode: "emergency" | "stable_only"`。
    * `emergency` 模式行为如当前设计。
    * `stable_only` 模式则只会找到并更新默认规则，而保留所有其他高优先级的规则不变。
3. **集成审批工作流 (Integration with Approval Workflows)**:
    * 引入一个“发布申请 (Release Proposal)”的概念。`PUT /releases` 不再直接生效，而是创建一个状态为 `pending_approval` 的 `ReleaseProposal`。
    * 通过 Webhook 通知外部系统（如 Jira, ServiceNow 或一个内部审批服务）。
    * 当收到外部系统的审批通过回调后，Conflux 才真正地将该 `ReleaseProposal` 应用为当前的发布策略。这使得 Conflux 可以无缝集成到企业现有的 ITIL/SOX 合规流程中。
