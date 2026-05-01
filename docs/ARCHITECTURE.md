# 架构

```text
React UI
  -> Tauri commands
    -> Rust application services
      -> core traits and domain models
        -> scanner / classifier / planner / executor / storage / skill / ai / platform
```

## 分层责任

- `apps/desktop/src`：中文 UI、页面状态、用户确认交互。
- `apps/desktop/src-tauri`：Tauri command 边界，二次校验危险请求。
- `crates/core`：共享领域模型和 trait。
- `crates/scanner`：只读扫描文件元数据。
- `crates/classifier`：生成分类结果和证据，不执行。
- `crates/planner`：生成可预览 Plan，不执行。
- `crates/executor`：只执行已确认 Plan。
- `crates/rollback`：生成和应用撤销记录。
- `crates/storage`：SQLite migration 与本地状态。
- `crates/skill-engine`：用户习惯规则加载、命中和保存。
- `crates/ai-gateway`：provider 抽象、脱敏、schema 校验。
- `crates/platform`：macOS/Windows capability 和路径差异。

## MVP 流程

```text
选择目录 -> scan_folder -> classify_files -> generate_plan -> review_plan -> execute_confirmed_plan -> rollback_batch
```
