# paper-master

Windows 本地优先论文管理器，当前版本 **v0.4.3**（2026-10-27）。使用 Tauri 2、React、TypeScript、Rust 和 SQLite。

## 下载与安装

普通用户下载 [GitHub Release](https://github.com/xiao-john-zhao/paper-master/releases) 中的 Windows x64 安装程序，双击即可安装，无需 Node.js、Rust 或 Python。安装器提供简体中文并包含 WebView2 离线运行时。仓库为私有，下载需要受邀账号。

完整说明：[中文下载安装使用指南](docs/下载安装使用指南.md)。分享 ZIP 包含安装器、使用指南、校验文件和可选的 LM Studio 联网安装脚本。

## 本版功能

- **编辑 AI 结果**：在论文详情选择分析版本后点击“编辑 AI 分析内容”，修改全部九类字段；保留生成原文和各模型版本，保存后更新搜索与文件缓存。
- **关系图局部查看**：滚轮与双击锚定缩放，坐标重绘而非图片放大；支持平移、完整标签换行、独立字号与节点大小、节点聚焦和局部关联筛选。
- **多服务接入**：LM Studio、Codex CLI、OpenAI 兼容 API 与演示模式。外部 API 独立保存地址、Bearer 密钥和模型，支持 JSON Object / JSON Schema，需要启用联网授权。
- **自适应论文分析**：整体语义基准 → 高参数直接分析 → 分级降参直接分析 → 全文重叠分段 → 分层综合 → 基准及原文证据复核。每篇重新从最高配置开始；详情展示实际参数、分段覆盖、引文和复核结论。
- **本地资料库**：PDF/文件夹导入、全文与关键词搜索、多关键词交集、作者与潜在团队关系、持久队列、多篇比较和独立版本结果。
- **持久恢复**：同一用户采用稳定数据目录，可更换论文缓存位置，启动恢复缺失记录，缓存保留人工修改、生成原文和分析处理报告。

直接分析使用有界章节材料；分段回退检查全部已提取文字的覆盖，并防止合并时丢失整个证据类别。语义复核是有界证据的一致性检查，不是正确性概率或“自动证明最优”。原文证据优先于粗略语义基准；无法通过校验时保留旧结果及失败原因，不伪造成功。

## 本地与在线使用

使用本地模型：单独安装 [LM Studio](https://lmstudio.ai/download?os=windows)、下载适合本机的模型、启动本地服务；在 Settings 保持 `http://127.0.0.1:1234/v1`，选择模型并加载测试。也可运行分享包或源码中的 `scripts/Install-LMStudio.cmd` 进行可选联网安装。LM Studio 和模型不随软件、Git 或源码迁移包分发。

使用外部 API：选择“其他 API（OpenAI 兼容）”，填写包含服务前缀的 HTTPS 地址、密钥及模型，选择服务支持的输出格式，开启联网授权并保存。应用支持 Chat Completions；其他原生协议需兼容网关。关闭联网后阻止外部 API；Codex 保留此前回退本地模型的行为。

导入的原 PDF 保留原位置；默认不发送论文到云端。只有明确启用联网并选择在线服务才会发送有界材料。论文、提示词、模型推理和密钥不写入诊断日志。分享安装包不包含个人论文或登录状态。

## 数据、升级与备份

同一 Windows 用户的数据默认位于 `%APPDATA%\app.papermaster.desktop\`，准确位置见 Diagnostics。论文缓存默认在 `library-cache\v1\`，可在 Diagnostics 更改；目录名不随版本变化。本次 JSON 文件格式升级为 2，保留读取格式 1 的兼容能力；已有数据库不重建。

升级前关闭应用并备份数据库和所选缓存目录；迁移个人资料库需另外复制缓存与原 PDF，再重新关联变化的路径。API 密钥在本机数据库中，不要分享个人数据库。

## 开发

需要 Windows 10/11、Node.js、项目固定的 pnpm、Rust MSVC 与 Visual Studio C++ Build Tools。使用项目锁文件。

```powershell
pnpm install --frozen-lockfile
pnpm test
pnpm build
cargo test --locked --manifest-path src-tauri/Cargo.toml
pnpm tauri dev
# 生成中文 NSIS 安装包（首次需要联网下载 WebView2 离线运行时）
pnpm tauri build --ci -- --locked
```

默认测试不依赖 LM Studio；实时外部测试为 ignored，需要显式授权才运行。浏览器预览是内存演示库。当前验证基线与限制见 [VALIDATION.md](VALIDATION.md)。

源码迁移包：`pwsh -File scripts/New-TransferPackage.ps1 -WithoutPaperCorpus`。迁移包与普通用户安装包分别使用；不要复制 node_modules、target、个人缓存或模型。

## 项目文档

- [中文下载安装使用指南](docs/下载安装使用指南.md)
- [架构与接口](ARCHITECTURE.md)
- [开发交接](CODEX_HANDOFF.md)
- [开发计划](DEVELOPMENT_PLAN.md)
- [验证记录](VALIDATION.md)
- [排错](DEBUGGING.md)
- [更新日志](CHANGELOG.md)

旧版本实施记录保留在 `docs/` 和 Git 历史中。下一维护目标为 v0.4.4。
