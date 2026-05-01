# 共享契约

Rust 类型以 `crates/core/src` 为唯一来源。前端类型应从 Tauri command payload 镜像生成或手工保持同名。

## 核心对象

- `FileItem`：扫描得到的文件元数据，不包含文件正文。
- `ClassificationResult`：分类候选、置信度、证据和风险。
- `OrganizationPlan`：所有可执行变化的唯一载体。
- `FileOperationPlan`：`CreateFolder`、`MoveFile`、`RenameFile`。
- `UserApproval`：用户确认凭据。
- `ExecutionBatch`：执行批次和状态。
- `RollbackEntry`：撤销记录。
- `Skill`：用户可见、可禁用的长期习惯规则。

## 危险 command

`execute_confirmed_plan` 必须同时收到：

- `OrganizationPlan`
- `UserApproval`
- validate 通过结果

缺任何一项都应拒绝执行。
