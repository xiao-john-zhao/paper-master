# Codex repository instructions

本文件适用于整个 `paper-master` 仓库。

## 接管顺序

开始修改前，依次阅读：

1. `README.md`
2. `CODEX_HANDOFF.md`
3. `ARCHITECTURE.md`
4. `DEVELOPMENT_PLAN.md`
5. 与任务相关的 `DEBUGGING.md`、`VALIDATION.md` 和 `CHANGELOG.md`

先运行 `git status --short --branch`。本次迁移整理产生的文档变更可能尚未提交；不得覆盖或丢弃用户已有改动。

## 不可破坏的项目约束

- 项目是 Windows 本地优先的论文管理器。不得把论文内容、完整提示词、模型推理或数据库内容写入日志或云端服务。
- 2026-10-23 用户明确要求新增 Codex 在线检索与阅读总结：仅在用户启用 `codex_online` 时，允许 Codex adapter 向在线服务提交自然语言检索需求、所选参考论文的有界题名/作者信息或有界论文证据。这是上述云端限制的明确例外；数据库、学校凭据、完整无界原文和模型推理仍不得发送，诊断日志仍不得记录论文和提示词。
- 导入的 PDF 必须原地保留；应用和测试不得移动、覆盖或删除源 PDF。
- LM Studio 仅允许回环地址。不要放宽为远程主机，也不要假设开发机已安装或正在运行 LM Studio。
- LM Studio 是可选外部运行时，不得把它的程序、模型、缓存、日志或本机设置加入仓库或迁移包。
- 数据库变更必须新增有序迁移并保留现有用户数据；不得通过静默重建数据库处理模式变化。
- React 视图不得直接访问 SQLite、解析 PDF 或发起 LM Studio 请求。保持 `React views -> application services -> domain interfaces -> adapters` 的依赖方向。
- AI 输出必须继续通过版本化结构、JSON/Schema 校验、简体中文检查和证据约束；不要使用或记录 reasoning 文本作为最终答案。

## 开发与验证

- 使用 `pnpm-lock.yaml` 和 `src-tauri/Cargo.lock`，不要无目的升级依赖。
- 不要复制或提交 `node_modules`、`target`、`dist`、`.pnpm-store`、`.cargo-home`、数据库、日志或 `.env` 文件。
- 常规改动至少运行相关测试。交接或发布前运行：

```powershell
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

- 默认测试不需要 LM Studio。实时 LM Studio 测试带有 `ignored` 标记，仅在用户明确要求并且本机服务可用时运行。
- 真实 PDF 语料测试必须保持只读并使用内存数据库。
- 发生失败时记录实际命令、退出码和首个可操作错误；不要把环境故障误写成产品回归。

## 文档和发布

- 行为、版本、测试基线或运行方式变化时，同步更新 `README.md`、`CHANGELOG.md`、`VALIDATION.md` 或 `CODEX_HANDOFF.md` 中相关部分。
- 当前版本为 `v0.4.1`，下一维护目标版本为 `v0.4.2`。不要在没有版本决策的情况下修改三个版本源：`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`。
- 使用 `scripts/New-TransferPackage.ps1` 生成迁移包。生成的 `handoff/` 目录不进入 Git。
