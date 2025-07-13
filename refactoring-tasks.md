# 代码重构与清理任务清单

## 项目概述

根据CONTRIBUTING.md对现有代码进行全面重构和清理，包括文件拆分、代码质量修复、文档完善等。

## 任务进度总览

- ✅ 已完成：4个任务
- 🔄 进行中：1个任务  
- ⏳ 待完成：39个任务

## 阶段1: 代码组织重构 🔄

**目标**: 拆分超过200行的文件，优化模块结构

### ✅ TASK-1.1: 拆分src/raft/node.rs (921行)

**状态**: 已完成  
**描述**: 将src/raft/node.rs拆分为多个模块：节点核心、资源限制、配置管理等

### ✅ TASK-1.2: 拆分src/raft/validation.rs (529行)

**状态**: 已完成  
**描述**: 将验证逻辑拆分为多个专门模块

### ✅ TASK-1.3: 拆分src/benchmarks/mod.rs (507行)

**状态**: 已完成  
**描述**: 将基准测试按功能分类拆分

### ⏳ TASK-1.4: 拆分src/raft/store/config_ops.rs (498行)

**状态**: 进行中  
**描述**: 将配置操作按CRUD功能拆分  
**备注**: 已清理相关文件，需要完成最终整理

### ⏳ TASK-1.5: 拆分其他超过200行的文件

**状态**: 待完成  
**描述**: 拆分剩余的15个超过200行的文件  
**文件列表**:

- src/raft/store/raft_storage.rs (487行)
- src/raft/store/store.rs (456行)
- src/raft/store/raft_impl.rs (445行)
- src/raft/client/mod.rs (398行)
- src/raft/types/config.rs (387行)
- src/raft/store/persistence.rs (378行)
- src/raft/state_machine.rs (356行)
- src/raft/store/raft_storage_v2.rs (354行)
- src/raft/store/commands/mod.rs (349行)
- src/raft/types/command.rs (334行)
- src/raft/store/delete_handlers.rs (329行)
- src/raft/types/mod.rs (324行)
- src/raft/store/types.rs (314行)
- src/raft/mod.rs (301行)
- src/raft/store/commands/version_commands.rs (298行)
- src/raft/store/transaction.rs (295行)

## 阶段2: 代码质量修复 ⏳

**目标**: 修复Clippy警告，清理死代码和未使用导入

### ⏳ TASK-2.1: 修复模块命名问题

**描述**: 修复模块与包含模块同名的问题 (module_inception)

### ⏳ TASK-2.2: 清理死代码和未使用导入

**描述**: 删除未使用的字段、方法和导入语句

### ⏳ TASK-2.3: 修复模式匹配问题

**描述**: 修复不必要的引用模式和冗余模式匹配

### ⏳ TASK-2.4: 修复性能相关警告

**描述**: 修复不必要的clone()、借用问题等

### ⏳ TASK-2.5: 修复代码风格问题

**描述**: 修复可派生的impl、手动范围检查等

### ⏳ TASK-2.6: 修复函数参数过多问题

**描述**: 重构超过7个参数的函数，使用结构体封装

## 阶段3: 文档完善 ⏳

**目标**: 添加缺失的文档注释，完善API文档

### ⏳ TASK-3.1: 添加模块级文档注释

**描述**: 为所有模块添加//!注释，说明设计意图

### ⏳ TASK-3.2: 添加公共API文档注释

**描述**: 为所有公共API添加///文档注释，包括示例代码

### ⏳ TASK-3.3: 完善错误类型文档

**描述**: 为错误类型添加详细的文档说明

### ⏳ TASK-3.4: 添加复杂逻辑注释

**描述**: 为复杂的业务逻辑添加详细注释

### ⏳ TASK-3.5: 生成和验证文档

**描述**: 使用cargo doc生成文档并验证完整性

## 阶段4: 测试改进 ⏳

**目标**: 分离测试文件，提高测试覆盖率

### ⏳ TASK-4.1: 分离单元测试文件

**描述**: 将嵌入式测试移动到独立的_test.rs文件

### ⏳ TASK-4.2: 创建集成测试

**描述**: 在tests/目录下创建集成测试

### ⏳ TASK-4.3: 添加属性测试

**描述**: 使用proptest添加属性测试

### ⏳ TASK-4.4: 提高测试覆盖率

**描述**: 添加缺失的测试用例，达到≥80%覆盖率

### ⏳ TASK-4.5: 优化异步测试

**描述**: 使用#[tokio::test]优化异步测试

## 阶段5: 性能优化 ⏳

**目标**: 优化性能问题，减少不必要的clone和内存分配

### ⏳ TASK-5.1: 优化内存分配

**描述**: 减少不必要的clone()和内存分配

### ⏳ TASK-5.2: 使用Cow优化

**描述**: 在适当的地方使用Cow处理借用/拥有数据

### ⏳ TASK-5.3: 优化迭代器使用

**描述**: 优先使用迭代器而非显式循环

### ⏳ TASK-5.4: 添加性能检查

**描述**: 启用#![deny(clippy::perf)]性能检查

## 阶段6: 安全加固 ⏳

**目标**: 添加安全检查，完善错误处理

### ⏳ TASK-6.1: 添加安全检查

**描述**: 启用#![deny(unsafe_code)]和#![deny(clippy::security)]

### ⏳ TASK-6.2: 完善错误处理

**描述**: 禁止unwrap()/expect()，使用thiserror+anyhow

### ⏳ TASK-6.3: 添加日志追踪

**描述**: 使用tracing而非log，添加#[instrument]

### ⏳ TASK-6.4: 安全审计

**描述**: 使用cargo audit检查依赖漏洞

## 执行指南

### 优先级

1. **高优先级**: 阶段1（代码组织重构）
2. **中优先级**: 阶段2（代码质量修复）
3. **低优先级**: 阶段3-6（文档、测试、性能、安全）

### 执行原则

- 每次只专注一个任务
- 完成后立即测试编译和运行
- 保持增量提交，便于回滚
- 遵循Rust最佳实践和CONTRIBUTING.md规范

### 验收标准

- 所有文件不超过200行
- cargo clippy无警告
- cargo test全部通过
- 测试覆盖率≥80%
- 文档完整且准确

---
*最后更新: 2025-01-11*
