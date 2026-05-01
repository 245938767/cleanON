# 安全与隐私边界

## 不可突破的执行边界

系统中的 AI、规则分类器、Skill 引擎和 Planner 都只能返回建议或 `OrganizationPlan`。只有 Executor 可以执行文件系统操作，而且 Executor 必须要求 `UserApproval`。

## 文件操作原则

- MVP 只允许创建目录、移动文件、重命名文件。
- 默认不删除文件。
- 执行前必须 validate。
- 每个执行批次必须生成 rollback 记录。
- 部分失败时保留可恢复状态，不吞掉错误。
- 测试只能在临时目录执行真实文件操作。

## 默认敏感目录

- `~/.ssh`
- `~/.gnupg`
- `~/Library/Keychains`
- `~/Library/Application Support/1Password`
- `~/Library/Application Support/Google/Chrome`
- `~/Library/Application Support/Firefox`
- `AppData/Roaming/Microsoft/Credentials`
- `AppData/Roaming/1Password`
- `AppData/Local/Google/Chrome/User Data`

## 云请求脱敏

发送到云 provider 前必须移除：

- 用户真实 home 路径
- 完整绝对路径
- API Key
- 原始文件正文
- 身份证、银行卡、合同正文等敏感内容

默认只发送相对路径、扩展名、大小、mtime、分类证据摘要。
