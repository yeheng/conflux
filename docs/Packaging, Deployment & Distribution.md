### **核心模块详细设计：打包、部署与分发**

该模块的目标是为不同的目标用户和环境，提供标准化的、可靠的、易于使用的交付产物。我们将主要关注两种交付形式：**容器镜像 (Container Images)** 和 **官方 Helm Chart**。

#### **1. 接口设计 (API Design)**

此模块不涉及运行时 API，它的“接口”是提供给用户的**交付产物**及其**配置方式**。

##### **a) 容器镜像 (Container Image)**

*   **镜像仓库:** `ghcr.io/conflux-project/conflux` (或 Docker Hub)。
*   **标签 (Tags):**
    *   `latest`: 指向最新的稳定版。
    *   `1.2.3`: 具体的语义化版本。
    *   `1.2`: 指向 `1.2.x` 系列的最新版。
    *   `edge`: 基于 `main` 分支的最新构建，用于测试。
*   **镜像暴露的端口:**
    *   `8080`: HTTP/REST API 端口。
    *   `8081`: gRPC API 端口。
    *   `9001`: Raft 内部通信端口。
    *   `9090`: 运维/管理 API 端口。
    *   `9595`: Prometheus `/metrics` 端口。
*   **环境变量 (Environment Variables) 配置:**
    *   `CONFLUX_NODE_ID`: 节点的 Raft ID。
    *   `CONFLUX_RAFT_ADDRESS`: Raft 通信地址。
    *   `CONFLUX_API_ADDRESS`: API 监听地址。
    *   `CONFLUX_DATABASE_URL`: 元数据数据库的连接字符串。
    *   `CONFLUX_RAFT_BOOTSTRAP_EXPECT`: (用于自动引导) 期望的初始集群大小。
    *   `RUST_LOG`: 日志级别，例如 `info,conflux=debug`。
*   **挂载点 (Volumes):**
    *   `/data`: 用于存储 RocksDB 数据的持久化目录。

##### **b) Helm Chart**

*   **Chart 仓库:** 一个可通过 `helm repo add ...` 添加的公共 Chart 仓库。
*   **Chart 名称:** `conflux/conflux`
*   **`values.yaml` (核心配置接口):**
    ```yaml
    # values.yaml
    replicaCount: 3

    image:
      repository: ghcr.io/conflux-project/conflux
      tag: "1.2.3"
      pullPolicy: IfNotPresent

    # 服务暴露方式
    service:
      type: ClusterIP
      port: 8080

    # Raft 和运维服务的 headless service
    headlessService:
      raftPort: 9001
      adminPort: 9090

    # 持久化存储配置
    persistence:
      enabled: true
      storageClassName: "gp2"
      size: 10Gi

    # 资源限制
    resources:
      limits:
        cpu: 1000m
        memory: 2Gi
      requests:
        cpu: 500m
        memory: 1Gi
    
    # 外部元数据数据库配置
    database:
      # useExistingSecret: my-db-secret
      url: "postgresql://user:pass@host:5432/conflux"

    # TLS 加密配置
    tls:
      enabled: true
      # useExistingSecret: my-tls-secret
      # 或者让 cert-manager 自动生成
      certManager:
        enabled: true
        issuer: letsencrypt-prod

    # 完整应用配置 (会被挂载为 configmap)
    config:
      log_level: "info"
      # ... 其他应用层配置 ...
    ```

---

#### **2. 出参入参设计 (Input/Output Parameter Design)**

此模块的设计 fokusiert sich auf die Artefakte und ihre Konfiguration, nicht auf Laufzeit-I/O.
*   **输入:**
    *   `Dockerfile` 和构建上下文。
    *   Helm Chart 的模板文件和 `values.yaml`。
    *   CI/CD 流水线的触发器（如 Git tag）。
*   **输出:**
    *   发布到容器镜像仓库的、多架构的容器镜像。
    *   打包好的 Helm Chart (`.tgz`)，发布到 Chart 仓库和 GitHub Releases。
    *   (可选) 预编译的二进制文件、`deb`/`rpm` 包。

---

#### **3. 数据模型设计 (Data Model Design)**

此模块的数据模型就是**交付产物的结构**。

##### **a) `Dockerfile` (多阶段构建)**

```dockerfile
# --- Stage 1: Builder ---
# 使用官方的 Rust 镜像作为构建环境
FROM rust:1.73 as builder

WORKDIR /usr/src/conflux
COPY . .

# 使用 cargo-chef 来缓存依赖，加快后续构建
# (此处省略 cargo-chef 的详细步骤)

# 编译 release 版本的二进制文件，开启所有生产 feature
RUN cargo build --release --features "protocol-http,protocol-grpc,kms-aws"

# --- Stage 2: Final Image ---
# 使用一个精简的、安全的 Linux 发行版作为基础镜像
FROM debian:bullseye-slim

# 安装必要的运行时依赖 (如 OpenSSL, CA 证书)
RUN apt-get update && apt-get install -y ca-certificates openssl && rm -rf /var/lib/apt/lists/*

# 从 builder 阶段复制编译好的二进制文件
COPY --from=builder /usr/src/conflux/target/release/conflux /usr/local/bin/conflux

# 创建一个非 root 用户来运行应用
RUN useradd --create-home --shell /bin/bash conflux
USER conflux
WORKDIR /home/conflux

# 暴露所有需要的端口
EXPOSE 8080 8081 9001 9090 9595

# 定义容器的启动命令
CMD ["conflux"]
```

##### **b) Helm Chart 结构**

```
conflux/
├── Chart.yaml          # Chart 的元数据
├── values.yaml         # 默认配置值
├── templates/
│   ├── _helpers.tpl    # 模板辅助函数
│   ├── deployment.yaml # (如果需要部署无状态组件)
│   ├── statefulset.yaml# **核心**: 用于部署 Conflux 节点
│   ├── service.yaml      # 用于客户端访问的 ClusterIP/LoadBalancer
│   ├── headless-service.yaml # 用于 Raft 内部节点发现
│   ├── configmap.yaml    # 用于挂载配置文件
│   ├── secret.yaml       # 用于管理数据库密码等
│   └── ... (其他 K8s 资源, 如 Ingress, ServiceAccount)
└── charts/             # (可选) 依赖的子 chart，如 Redis
```

---

#### **4. 核心流程设计 (Core Flow Design)**

##### **a) CI/CD 发布流程 (以 GitHub Actions 为例)**

```mermaid
graph TD
    A[Developer pushes a new tag<br>e.g., `v1.3.0`] --> B{Trigger `release.yml` workflow};
    
    subgraph B [Release Workflow]
        C[Setup Go, Rust, Helm, Docker] --> D{Run tests & lint};
        D -- Success --> E[Build & Push Multi-Arch Image];
        subgraph E
            E1[Login to ghcr.io] --> E2[Setup QEMU & buildx];
            E2 --> E3[Build for linux/amd64 and linux/arm64];
            E3 --> E4[Push images with version and `latest` tags];
        end
        
        E --> F[Package & Publish Helm Chart];
        subgraph F
            F1[Update Chart.yaml version] --> F2[helm package .];
            F2 --> F3[Push .tgz to GitHub Releases];
            F3 --> F4[Update Helm repo index.yaml and push to `gh-pages` branch];
        end

        F --> G[Create GitHub Release];
        G --> H{Post notification to Slack/Teams};
    end
    
    B -- Failure --> I{Notify failure};
```

##### **b) 用户使用 Helm Chart 的部署流程**

```mermaid
sequenceDiagram
    participant User as User/SRE
    participant HelmCLI as helm
    participant K8s as Kubernetes API Server
    participant ConfluxPods as Conflux Pods

    User->>HelmCLI: helm repo add conflux ...
    User->>HelmCLI: helm repo update
    User->>HelmCLI: helm install my-conflux conflux/conflux -f my-values.yaml
    
    HelmCLI->>K8s: Create StatefulSet, Services, ConfigMap, etc.
    K8s->>K8s: Schedule Pods based on StatefulSet spec
    K8s->>ConfluxPods: Start conflux-0, conflux-1, conflux-2
    
    Note over ConfluxPods: Pods start and begin the<br>automatic cluster bootstrapping process.
    
    User->>HelmCLI: helm status my-conflux
    HelmCLI->>K8s: Get status of created resources
    K8s-->>HelmCLI: Return status
    HelmCLI-->>User: Display deployment status
```

---

#### **5. 关键逻辑详细说明 (Key Logic Details)**

##### **a) 多阶段 Dockerfile**

使用多阶段构建是容器化 Rust 应用的最佳实践。
*   **`builder` 阶段:** 包含完整的 Rust 工具链和所有源代码，体积巨大（通常 >1GB）。它的唯一目的是编译出一个静态链接或动态链接的二进制文件。
*   **`final` 阶段:** 从一个非常精简的基础镜像开始（如 `debian-slim` 或 `gcr.io/distroless/cc-debian11`），只安装最少的运行时依赖，然后从 `builder` 阶段复制编译好的二进制文件。最终的镜像体积可以被优化到 100MB 以下，并且包含的攻击面更小。

##### **b) `StatefulSet` 的使用**

对于像 Conflux 这样的分布式有状态应用，必须使用 `StatefulSet` 而不是 `Deployment`。
*   **稳定的网络标识:** Pod 会有可预测的主机名（`conflux-0`, `conflux-1`），这对于 Raft 的节点发现至关重要。
*   **稳定的持久化存储:** 每个 Pod 会绑定到一个独立的 `PersistentVolumeClaim`。当 Pod 被重建时，它会重新挂载到同一个 PVC，从而保留其 RocksDB 数据。
*   **有序的部署和缩放:** `StatefulSet` 保证了 Pod 会按顺序（0, 1, 2...）创建和销毁，这对于安全地进行集群成员变更非常有帮助。

##### **c) Helm Chart 的灵活性**

`values.yaml` 的设计应该非常灵活。例如，对于 secrets，应该同时支持：
1.  直接在 `values.yaml` 中提供明文字符串（仅用于测试）。
2.  引用一个已经存在的 Kubernetes `Secret`（生产环境推荐）。
3.  让 Chart 自动创建一个新的 `Secret`。

这通过在模板中使用 `if/else` 逻辑来实现。

---

#### **6. 详细测试用例和测试方法 (Detailed Test Cases & Methods)**

##### **a) CI/CD 流水线测试**

*   **Linting:** 在 CI 中运行 `helm lint ./chart` 来检查 Chart 的语法和最佳实践。
*   **Dry-Run Installation:** 在 CI 中运行 `helm install --dry-run --debug ...` 来渲染模板，并验证生成的 Kubernetes YAML 是否符合预期。
*   **Integration Test:** (高级) 在 CI 中使用 `kind` (Kubernetes in Docker) 或 `minikube` 启动一个临时的 K8s 集群，然后真实地 `helm install` 我们的 chart，并运行一些基本的健康检查来验证部署是成功的。

##### **b) 手动测试**

*   在多种 Kubernetes 环境（GKE, EKS, AKS, 本地 kind）中测试 Helm Chart 的部署。
*   测试所有主要的 `values.yaml` 配置项，特别是服务类型（`ClusterIP` vs `LoadBalancer`）、持久化（开启/关闭）、TLS（开启/关闭）等。

---

#### **7. 设计依赖 (Dependencies)**

*   **Docker / buildx**: 用于构建容器镜像。
*   **Helm**: 用于打包和部署 Kubernetes 应用。
*   **CI/CD Platform (e.g., GitHub Actions)**: 自动化整个构建和发布流程。
*   **Kubernetes Cluster**: 最终的部署目标环境。
*   **Container Registry (e.g., GHCR, Docker Hub)**: 存储和分发镜像。

---

#### **8. 已知存在问题 (Known Issues)**

1.  **基础镜像的安全漏洞**: `debian-slim` 等通用基础镜像仍然可能包含未被发现的安全漏洞 (CVEs)。
2.  **Helm 升级的复杂性**: 对于有状态应用，`helm upgrade` 可能是一个复杂的操作。如果升级涉及到不兼容的状态变更，可能需要手动的数据迁移步骤，或者一个更智能的 Operator 来处理。
3.  **多架构构建的复杂性**: 维护一个能稳定构建 `amd64` 和 `arm64` 镜像的 CI/CD 流程需要一些额外的配置（如 QEMU）。

---

#### **9. 可迭代 Enhancement (Potential Enhancements)**

1.  **使用 Distroless 镜像**: 迁移到 Google 的 Distroless 镜像 (`gcr.io/distroless/cc-debian11`)。这些镜像只包含应用及其运行时依赖，不包含 shell、包管理器或任何其他程序，极大地减小了攻击面。
2.  **开发 Conflux Operator (最终形态)**: 如之前在运维模块中提到的，开发一个 Kubernetes Operator 是管理有状态应用的终极解决方案。Operator 可以编码复杂的升级逻辑（例如，先升级一个 Follower，观察，再升级下一个），处理备份、恢复和故障转移，使得 Conflux 在 Kubernetes 上的运维体验如同云原生数据库一样丝滑。
3.  **提供多种安装方式**:
    *   为非 Kubernetes 用户提供 **Docker Compose** 文件，让他们可以快速地在单机上启动一个测试集群。
    *   为裸金属或 VM 用户提供 **Ansible Playbook** 或 **Terraform Module**，以自动化节点的配置和应用的部署。
4.  **发布软件物料清单 (SBOM)**: 在每次发布时，自动生成并附加一份 SBOM (Software Bill of Materials)，列出应用及其所有依赖项的版本。这对于满足供应链安全和合规性要求越来越重要。