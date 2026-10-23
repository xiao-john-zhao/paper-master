# 本机部署与使用

本机源码目录：`D:\paper-master\paper-master`。保留迁移包、Git 历史和交接时已有的未提交文档改动。

## 启动

2026-10-23 更新：本机版增加 Codex 联网检索/阅读总结、作者关系、摘要修复和 PDF 打开修复。使用 Codex 时可直接打开 `application\\paper-master-local.exe`，或在下述命令添加 `-SkipLocalModel` 以跳过 LM Studio 初始化。学校订阅下载为试验功能，见 `问题清单/2026-10-23-联网检索与下载问题清单.md`。学校凭据只在 `%LOCALAPPDATA%\\paper-master-private\\webvpn.txt`，不在本项目内。

推荐在 PowerShell 7 中执行：

本机还提供桌面快捷方式 **paper-master（本机版）**，调用相同的启动脚本。

```powershell
cd D:\paper-master\paper-master
pwsh -NoProfile -File .\scripts\Start-Local.ps1
```

脚本检查 `127.0.0.1:1234` 的 LM Studio 服务，必要时通过已安装的 `lms` 启动服务和加载本地语言模型，再打开 `application\paper-master-local.exe`。不会下载模型，不会打开局域网监听或 CORS。已加载模型直接复用；首次加载默认上下文为 24,576 token、单路推理。

可指定模型和加载上下文：

```powershell
pwsh -NoProfile -File .\scripts\Start-Local.ps1 -Model 'qwen/qwen3.8-27b' -ContextLength 24576
pwsh -NoProfile -File .\scripts\Start-Local.ps1 -CheckOnly
```

已启动 LM Studio API 时，也可以直接打开 `application\paper-master-local.exe`。NSIS 安装包在 `src-tauri\target\release\bundle\nsis\`；安装包和可执行文件可从源码重建。本次增强未创建新发布标签，应用内部版本仍为 `0.3.1`。

本轮最新便携程序为 `application\\paper-master-updated.exe`，启动脚本会优先选择它。由于用户停止了桌面操作，当前旧窗口保持原状；首次使用新版请关闭旧窗口后再启动。旧的 `paper-master-local.exe` 保留，没有在运行期间覆盖。

## 本机配置

- Windows、Node 24.14.0、Rust/Cargo stable MSVC 1.98.0、VS 2022 C++ Build Tools、Windows SDK 和 WebView2 已检查。
- 使用现有 pnpm 11.25.0 按原 `pnpm-lock.yaml` 恢复依赖，没有升级项目锁文件。项目声明的包管理器版本仍为 11.16.0。
- 已有模型：Qwen3.8 27B Q4_K_M。32 GB 内存和 RTX 4070 SUPER 12 GB 下需要 CPU/GPU 混合运行，长论文处理可能需要数分钟。
- 应用 Settings：LM Studio、回环 URL、自动导入分析开启、温度 0.15、推理强度关闭额外推理（API `none`）、上下文 24,576、输入预算 24,000 字符、超时 600 秒。输出预算可设为 4,096 token。
- LM Studio 服务仅绑定 `127.0.0.1:1234`；启动 LM Studio 时自动启动 API，关闭敏感正文和逐 token 日志。原服务配置备份位于项目外层 `.deployment-backups`，不进入项目迁移包。

## 工作流

1. 点击 Import PDF 或 Folder。PDF 原地保留，新论文提取文本后进入队列，完成后自动保存中文问题、场景、方法、创新、贡献、结果、局限与概括。
2. 状态会自动刷新。等待和运行中的任务可取消；失败时查看详情页原因并点击 AI 分析重试。重复导入会跳过；扫描件需先 OCR。
3. 勾选 2–8 篇论文并点击 Compare。缺失分析会先补齐，再由本地模型结合内容分析与原文摘录生成语义差异，结果页附逐篇可核对的原文证据。
4. 参数、模型不兼容或证据校验失败会显示错误，不会替换为关键词结果。需要时可在 Settings 切换 Mock 做演示，Mock 结果带明确标记。

比较使用有界证据片段，不代表模型已逐页阅读每份完整 PDF；对于未报告的条件、不可直接比较的指标，结果要求明确说明证据不足。

## 重建

```powershell
pwsh -NoProfile -File .\scripts\Build-Local.ps1
# 只生成应用，不生成安装包：
pwsh -NoProfile -File .\scripts\Build-Local.ps1 -NoInstaller
```

构建脚本执行冻结锁文件安装、前后端测试和 Tauri 生产构建，并将应用复制到 `application\paper-master-local.exe`。重建前关闭已启动的本机应用。

个人应用数据库位于 `%APPDATA%\app.papermaster.desktop\paper-master.db`。迁移包不包含旧电脑个人库；本次验证不会从旧库重建或覆盖个人记录。真实 PDF 自动化语料测试使用内存数据库，完整验证结果见 `VALIDATION.md`。
