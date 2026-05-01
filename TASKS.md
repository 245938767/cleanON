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

## 当前完成情况

- 已锁定 MVP 范围：项目骨架 + 最小闭环。
- 已锁定首页 UI：中文优先、CleanMyMac 参考风格、两个大图标入口、无传统按钮。
- 已锁定安全边界：Plan-first、确认后执行、本地优先、云请求脱敏。
- 已完成 monorepo、Rust crates、Tauri command、前端 MVP、SQLite migration、Skill/AI MVP、CI 骨架。
- 验证通过：`cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`pnpm --filter desktop lint/test/build`、`bash scripts/ci-rust.sh`、`bash scripts/ci-frontend.sh`。
- 阶段 2 已补齐真实目录选择、扫描持久化、取消扫描和前端真实扫描结果展示。
- 阶段 3 已补齐规则分类、关键词分类、Skill 命中提升、基于扫描任务的 Tauri 分类入口和前端分类证据展示。
