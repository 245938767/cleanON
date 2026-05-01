# 智能文件整理项目代理契约

## 项目目标

本项目实现一款跨 macOS 与 Windows 的智能文件与桌面整理程序。第一轮交付聚焦 MVP 闭环：选择目录、扫描文件、生成整理方案、用户确认、执行文件操作、撤销、沉淀 Skill。

## 核心红线

- AI、规则引擎、分类器和 Planner 只能生成 `OrganizationPlan`，不能直接执行文件操作。
- 所有移动、重命名、创建目录、桌面整理动作必须先展示给用户确认。
- Executor 只接受已确认的 Plan，并且必须生成 rollback 记录。
- 默认本地优先；云模型必须由用户显式启用。
- 云端请求必须脱敏，不上传完整绝对路径、API Key、原始文件正文或敏感目录内容。
- API Key 不允许写入 SQLite、日志、前端状态、localStorage 或明文配置文件。
- 敏感目录默认不扫描。
- Windows 第一轮不写回真实桌面图标坐标，只做预览与文件归档。
- macOS 第一轮不承诺桌面图标坐标级排布。

## Subagent 协作协议

- 主线程负责基础设施、任务列表、集成和最终验证。
- 子代理只在分配的写入范围内修改文件，不重写全局计划。
- 子代理完成后必须报告 changed files、完成项、阻塞项和验证命令。
- 共享类型以 `crates/core` 和 `docs/CONTRACTS.md` 为准。
- 如果需要扩大写入范围，先向主线程报告，不要自行改动相邻模块。

## UI 方向

首页中文优先，参考 CleanMyMac 的友好现代工具感，但不复制品牌、图标、配色或文案。第一屏只保留两个大图标入口：

- `文件整理`
- `桌面整理`

不使用传统可见文本按钮，入口模块本身就是点击目标。复杂业务放到二级页面。

## 验证要求

修改后按风险运行：

- Rust：`cargo fmt --check`、`cargo clippy --workspace --all-targets`、`cargo test --workspace`
- Frontend：`pnpm --filter desktop lint`、`pnpm --filter desktop test`、`pnpm --filter desktop build`
- 全局：`pnpm lint`、`pnpm test`、`pnpm build`

测试不得依赖真实用户桌面、下载、文档目录。文件操作测试只能使用临时目录。
