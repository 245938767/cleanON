# 共享契约

`crates/core/src` 是 Rust 内部核心契约的唯一来源；Tauri command 对前端暴露的是 `*Dto`
边界类型，不直接暴露 core enum 的 Rust 命名或 serde casing。

## 核心对象

- `FileItem`：扫描得到的文件元数据，不包含文件正文。
- `ClassificationResult`：分类候选、置信度、证据和风险，只在 Rust 内部流转。
- `OrganizationPlan`：所有可执行变化的唯一内部载体。
- `FileOperationPlan`：内部操作枚举，覆盖 `CreateFolder`、`MoveFile`、`RenameFile`。
- `UserApproval`：执行前由 DTO 转换出的确认凭据。
- `ExecutionBatch`：执行批次和状态。
- `RollbackEntry`：撤销记录。
- `Skill`：用户可见、可禁用的长期习惯规则。

## Tauri DTO 边界

前端消费以下稳定 DTO：

- `GeneratePlanRequestDto`：`taskId/rootPath/mode` 为必填；`classifications` 可选。未提供分类时，Tauri
  层必须从扫描存储读取文件、重新分类，再交给 planner。
- `ClassificationInputDto`：前端如传分类结果，只传稳定字段 `fileId/categoryKey/confidence/evidence/risk`；
  Tauri 层按 `fileId` 从扫描存储恢复 `FileItem`，不得要求前端构造 core `ClassificationResult`。
- `OrganizationPlanDto`：计划预览边界，包含 `planId/taskId/rootPath/mode/rows/summary/createdAt`。
- `OperationRowDto`：每行必须包含 `operationId/operationType/title/source/target/reason/risk/selected/editableTarget/validationIssues/conflictStatus`。
- `UserApprovalDto`：前端确认边界，转换为 core `UserApproval { approved, approved_plan_id, approved_at, actor }` 后才能执行。
- `ExecutionBatchDto`、`HistorySummaryDto`：执行和历史摘要边界，使用稳定字符串状态。
- `SkillDto`、`SkillUpdateProposalDto`：Skill 列表与保存边界。
- `ModelSettingsDto`：模型设置边界，只允许 provider、cloudEnabled、model；不得包含 API Key、secret 或 token。

DTO 中的 `mode`、`operationType`、`risk`、`status` 使用稳定小写字符串，例如
`by_category`、`move_file`、`low`、`completed`。前端不要依赖 Rust enum 变体名。

## 危险 command

`execute_confirmed_plan` 是危险 command，必须同时收到：

- `OrganizationPlanDto`
- `UserApprovalDto`
- executor 重新 validate 通过的结果

Tauri 层必须只把 `selected = true` 的行转换为 core operation，并使用 `editableTarget` 作为最终目标路径。
缺少确认、plan id 不匹配、validate 不通过、路径逃逸 root、目标冲突或 source 不存在，都必须拒绝执行。

AI、规则引擎、分类器和 planner 只能生成计划 DTO 或 core plan；任何移动、重命名、创建目录动作只能在
`execute_confirmed_plan` 收到确认并通过验证后发生。
