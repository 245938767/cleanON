# 智能文件整理 MVP 任务列表

| 状态 | 任务 | Owner | 写入范围 | 验收证据 |
| --- | --- | --- | --- | --- |
| 完成 | 主线程基础设施 | Main | `AGENTS.md`, `.codex/skills/**`, `TASKS.md`, root config, Tauri shell | workspace 配置、约束文档、Tauri command 外壳存在；`cargo test --workspace` 通过 |
| 完成 | Rust Flow | Cicero | `crates/core/**`, `crates/scanner/**`, `crates/classifier/**`, `crates/planner/**`, `crates/executor/**`, `crates/rollback/**`, `crates/platform/**` | `cargo test --workspace` 覆盖扫描、计划、确认执行、撤销 |
| 完成 | Storage/Skill/AI | Epicurus | `crates/storage/**`, `crates/skill-engine/**`, `crates/ai-gateway/**`, `migrations/**` | migration、脱敏、Skill 测试通过 |
| 完成 | Frontend | Pauli | `apps/desktop/package.json`, `apps/desktop/src/**`, Vite/TS config | 中文双入口首页和二级流程可构建；浏览器截图已复核 |
| 完成 | QA/CI | Carson | `.github/**`, `scripts/**`, `docs/testing.md` | CI matrix 与本地验证命令存在；`scripts/ci-rust.sh` 和 `scripts/ci-frontend.sh` 通过 |
| 完成 | 阶段 2：扫描器与本地数据库 | Main | `TASKS.md`, `crates/core/**`, `crates/scanner/**`, `crates/storage/**`, `apps/desktop/src-tauri/**`, `apps/desktop/src/**`, `migrations/**` | 用户选择目录后真实扫描；结果写入 SQLite `file_item`；前端展示文件名、扩展名、大小、修改时间；扫描可取消；`cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`pnpm --filter desktop test/build` 通过 |
| 完成 | 阶段 3：分类引擎与规则引擎 | Main | `TASKS.md`, `crates/core/**`, `crates/classifier/**`, `apps/desktop/src-tauri/**`, `apps/desktop/src/**` | 基础分类、关键词规则、ClassificationRule、Skill 命中提升、分类结果 UI 已完成；`cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`pnpm --filter desktop test/build` 通过 |
| 进行中 | 共享契约基础层 | Contract Agent | `crates/core/**`, `apps/desktop/src-tauri/**`, `docs/CONTRACTS.md`, narrow `crates/storage/**` | Tauri DTO 与前端/Rust 契约对齐；Plan row 支持 Diff、确认、编辑、验证 |
| 待办 | 阶段 4：AI Provider 抽象层 | AI/Models Agent | `crates/ai-gateway/**`, `crates/storage/**`, `apps/desktop/src-tauri/**` | Mock/Ollama/OpenAI-compatible provider、schema 校验、模型设置命令、API Key 不落库 |
| 待办 | 阶段 5：整理计划 Plan 与可视化 Diff | Plan/Execute/History Agent + Frontend Agent | `crates/planner/**`, `apps/desktop/src/**`, `apps/desktop/src-tauri/**` | 用户能生成 Plan、查看 Diff、接受/拒绝/修改目标；未确认前不执行 |
| 待办 | 阶段 6：事务执行器与撤销系统 | Plan/Execute/History Agent | `crates/executor/**`, `crates/rollback/**`, `crates/storage/**`, `apps/desktop/src-tauri/**` | 确认后执行；执行批次和 rollback 持久化；历史页可撤销 |
| 待办 | 阶段 7：Skill 长期记忆系统 | Skill Agent + Frontend Agent | `crates/skill-engine/**`, `crates/classifier/**`, `crates/storage/**`, `apps/desktop/src/**`, `apps/desktop/src-tauri/**` | 用户操作事件可生成 Skill；Skill 可保存/禁用/删除；分类优先命中 |
| 待办 | 阶段 8：桌面整理 MVP | Desktop Platform Agent + Frontend Agent | `crates/platform/**`, `crates/planner/**`, `apps/desktop/src/**`, `apps/desktop/src-tauri/**` | macOS Desktop 文件归档建议；Windows 分区预览；不写真实桌面坐标 |
| 待办 | 阶段 9：本地模型与云模型完整接入 | AI/Models Agent + Frontend Agent | `crates/ai-gateway/**`, `crates/storage/**`, `apps/desktop/src/**`, `apps/desktop/src-tauri/**` | 本地优先；云端显式启用；请求脱敏；连接测试可用 |
| 待办 | 阶段 10：测试、打包、发布准备 | Main | `scripts/**`, `.github/**`, `docs/testing.md`, package config | Rust/Frontend 全量验证通过；浏览器主流程复核；剩余发布风险记录 |

## 当前完成情况

- 已锁定 MVP 范围：项目骨架 + 最小闭环。
- 已锁定首页 UI：中文优先、CleanMyMac 参考风格、两个大图标入口、无传统按钮。
- 已锁定安全边界：Plan-first、确认后执行、本地优先、云请求脱敏。
- 已完成 monorepo、Rust crates、Tauri command、前端 MVP、SQLite migration、Skill/AI MVP、CI 骨架。
- 验证通过：`cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`pnpm --filter desktop lint/test/build`、`bash scripts/ci-rust.sh`、`bash scripts/ci-frontend.sh`。
- 阶段 2 已补齐真实目录选择、扫描持久化、取消扫描和前端真实扫描结果展示。
- 阶段 3 已补齐规则分类、关键词分类、Skill 命中提升、基于扫描任务的 Tauri 分类入口和前端分类证据展示。
