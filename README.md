# v0.4.0 当前入口（2026-10-24）

启动 `application/paper-master.exe` 或 `pwsh -File scripts/Start-Local.ps1`。仅提供一个应用，联网在 Settings 开关中控制并保存；原有 local/updated 文件仅保留为回退备份。支持多模型结果保留/切换、全部整理文字和元数据编辑、同义关键词归一及潜在同团队关系图。

已将 `F:\论文` 的 439 个 PDF 原地引用归档并逐文件校验；全部有文字，439 份原文件 SHA-256 不变。含 143 条待人工核对记录，详见本机 `data/validation/review-needed.md`。本地摘录明确标为“非模型”，完整大模型分析只做了真实样本验证。

完整修改、限制、测试和回退说明：[本次修改记录](docs/2026-10-24-v0.4.0-unified-archive.md)。下文较早日期内容保留为历史背景，当前行为以上述记录为准。

# paper-master

`paper-master` 是一个 Windows 本地优先的研究论文管理桌面应用，使用 Tauri 2、React、TypeScript、Rust 和 SQLite 构建。当前版本为 `v0.4.0`，下一维护目标为 `v0.4.1`。

导入的 PDF 始终保留在原位置；应用不会移动或删除原文件。论文元数据、分析结果和诊断信息保存在本机。项目支持通过本机 LM Studio 服务执行 AI 分析，但 LM Studio 是可选的外部运行时，不属于项目依赖包。

## 当前功能

### 本轮增强（2026-10-23）

- 新增各主要耗时操作的进度条和全局任务中心：导入按文件数、下载按响应提供的总字节数显示；模型分析、对比、检索、模型加载、关系图和导出显示实际阶段、用时及完成/失败/取消状态。无法确定总量时显示循环进度，不推算模型生成百分比。
- 调整导航、论文标题、摘要、分析正文、表单和按钮的字号、行距与留白；长标题自动换行，窄窗口保持滚动与操作入口可用。
- 侧栏新增“论文关系图”：关键词网络按同篇论文共现连线，论文网络按共同关键词或共同署名连线；支持节点数/关系门槛筛选、作者关联开关、查找高亮、缩放和平移，点击节点或连线可核对关联论文并打开详情。支持带完整图例的 SVG 与高清 PNG 导出。图谱使用完整本地库的已有标签及作者，不需要模型或联网；关键词不足时先完成论文分析。共同署名不自动视为同一作者身份或已确认同组。
- 修复 Open PDF 的路径授权问题，由后端按论文 ID 验证并使用系统阅读器打开；文件迁移后可重新关联原 PDF。
- 新增“联网检索论文”：支持自然语言，例如“帮我查找与这篇论文相同课题组、关于多无人机 ISAC 公平资源分配的相似文章”。选择可选参考论文后，Codex 先解析主题、目标作者、团队关系和年份，再组合检索式搜索 IEEE；结果显示需求理解、匹配理由与来源。缺少参照时提示补充，不会将只有主题相似的结果冒充同组论文。提供公开 PDF 下载及学校 WebVPN 入口。学校订阅自动下载仍为试验功能，具体限制见 [问题清单](问题清单/2026-10-23-联网检索与下载问题清单.md)。
- Settings 可选择 **Codex CLI（在线阅读与总结）**，先勾选联网授权并保存，再执行论文分析。已有 LM Studio 与 Mock 模式继续可用；Codex 使用本机登录状态，需 `codex login`，也可通过 `PAPER_MASTER_CODEX_PATH` 指向 `codex.exe`。
- Codex 分析发送有界论文文本到在线服务，输出仍经过版本化结构、中文和证据检查；检索发送自然语言需求及所选参考论文的题名/作者信息，不发送参考论文全文。学校凭据不发送给 Codex。
- 英文摘要在关键词、作者脚注和机构说明之前结束；迁移 6 修复已有摘要。作者、第一作者、通信作者及原文证据独立显示，并列出本地库中同署名及潜在合作团队论文。
- 学校凭据 TXT 与浏览器资料只保存在当前用户的 `%LOCALAPPDATA%\\paper-master-private\\`，位于工程外；应用和迁移包不携带凭据。联网检索页提供本地 TXT 查看入口。迁移到其他电脑后需单独配置并重新登录。

无需初始化 LM Studio 时可直接启动 `application/paper-master.exe`，或运行 `pwsh -File scripts/Start-Local.ps1 -SkipLocalModel`。本轮仍保持 0.3.1 版本源，不创建发布标签。

- 递归导入 PDF/文件夹、重复检测、容错文本提取、元数据回退和逐文件诊断；
- 首次导入自动进入持久化本地 AI 分析队列，生成中文内容概括；批量任务串行执行，支持取消、失败后重试和重启续跑；
- SQLite 持久化、版本化迁移、CRUD、FTS5 关键词搜索、筛选和标签；
- Mock AI 开发模式，可在没有 LM Studio 的环境中测试主要界面与数据流程；
- 可选的本机 LM Studio 模型发现、显式加载、中文结构化分析、取消和 JSON 校验/修复；
- 有证据约束的受管标签，以及 2–8 篇论文的本地模型深度比较：重新综合已存分析和有界原文证据，比较问题、场景、方法、贡献、结果和局限，校验逐篇原文引文；
- 统一错误码、请求 ID、隐私安全的 JSONL 诊断日志和 Diagnostics 页面。

## 技术栈与目录

```text
paper-master/
├─ src/                 React/TypeScript 前端
├─ src-tauri/           Rust/Tauri 后端、SQLite、PDF 与 AI 适配器
├─ tests/               前端单元与服务测试
├─ assets/              源资源
├─ paper/               本地真实 PDF 测试语料（不进入 Git，可随迁移包携带）
├─ application/         发布产物说明；可执行文件可由源码重建
├─ scripts/             验证与迁移辅助脚本
├─ AGENTS.md            Codex 的仓库级工作约束
└─ CODEX_HANDOFF.md     当前开发状态与跨电脑接管说明
```

架构依赖方向和扩展点见 [ARCHITECTURE.md](ARCHITECTURE.md)，版本计划见 [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md)。

## 新电脑快速开始

本机接管后的直接启动方式与已验证配置见 [LOCAL_DEPLOYMENT.md](LOCAL_DEPLOYMENT.md)。本次功能增强在现有 `v0.3.1` 工作树上实现，尚未创建 `v0.3.2` 发布标签，三个版本源保持一致。

### 1. 安装开发环境

- Windows 10/11 和 WebView2；
- Node.js 20 或更高版本；
- pnpm `11.16.0`（`package.json` 中已固定包管理器版本）；
- Rust stable MSVC 工具链；
- Visual Studio Build Tools，并安装“使用 C++ 的桌面开发”工作负载。

如果系统提供 Corepack：

```powershell
corepack enable
corepack prepare pnpm@11.16.0 --activate
```

如果没有 Corepack：

```powershell
npm install --global pnpm@11.16.0
```

### 2. 解压和安装依赖

将迁移包解压到较短的本地路径，例如 `D:\dev\paper-master`。不要复制旧电脑的 `node_modules`、Rust `target`、pnpm/Cargo 缓存；它们体积大、包含机器相关文件，而且可以从锁文件重建。

```powershell
cd D:\dev\paper-master
git status
pnpm install --frozen-lockfile
```

迁移包保留了 Git 历史。接管时出现的文档类未提交变更来自本次交接整理，详情见 `CODEX_HANDOFF.md`。

### 3. 验证基线

```powershell
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

默认测试不需要 LM Studio。两个需要本机 LM Studio 的 Rust 实时烟雾测试标记为 `ignored`，不会在上述命令中运行。

### 4. 启动开发版

```powershell
pnpm tauri dev
```

前端单独调试可使用 `pnpm dev`。生产安装包通过 `pnpm tauri build` 生成到 `src-tauri/target/release/bundle/nsis/`。

## LM Studio（可选，不随迁移包提供）

未安装 LM Studio 时，选择 Settings 中的 **Mock (development)** 即可进行大部分开发和测试。

如需验证真实本地模型：

1. 单独安装 LM Studio、下载兼容模型并启动本地服务；
2. 在 paper-master 的 Settings 中保持回环地址（默认 `http://127.0.0.1:1234/v1`）；
3. 选择“加载并测试”并保存模型；保持“首次导入后自动分析”勾选，导入 PDF 后无需再点击分析按钮。
4. 勾选 2–8 篇论文并点击 Compare；应用会先补齐缺失的内容分析，再向本机模型提交结构化分析和有界原文摘录，生成六个维度的语义差异与原文证据。

只有新导入且提取到文本的论文会自动排队；重复导入不重复分析。单篇失败不阻断后续队列，详情页显示原因并允许重试。空白、扫描型或无法提取文本的 PDF 保留为库记录，需先进行 OCR 或修复 PDF。关闭应用会中断正在执行的请求；下次启动时恢复未完成任务。比较材料有字符预算，不能替代逐页阅读全文；不同数据集和实验条件的指标不直接排名。

Settings 可调整模型加载上下文、推理强度和输出预算。OpenAI 兼容接口的关闭推理值为 `none`（与模型能力列表中的 `off` 不同）。上下文参数用于下次加载；已加载模型需要先在 LM Studio 卸载再重新加载。

适配器拒绝非回环地址。迁移包不会包含 LM Studio 程序、模型、缓存、日志或旧电脑上的 LM Studio 设置。详细排错方法见 [DEBUGGING.md](DEBUGGING.md)。

## 本地数据与隐私

Windows 上的 SQLite 数据库和 JSONL 诊断日志位于标识符 `app.papermaster.desktop` 对应的 Tauri 应用数据目录。Diagnostics 页面会显示准确路径。

以下内容默认不进入 Git，也不会进入迁移包：

- `.env`、数据库和应用日志；
- `node_modules`、Rust `target`、pnpm/Cargo 缓存和 `dist`；
- LM Studio 的程序、模型、缓存、日志和设置；
- 历史发布 `.exe`/`.lnk` 文件。

`paper/` 中的 12 篇 PDF 是本项目的本地只读测试语料，默认包含在本次迁移包中，但不提交到 Git。应用中已经导入的个人数据库不会迁移；如需保留实际论文库状态，应另外备份 Tauri 应用数据目录，并在新电脑上修正原 PDF 路径。

## 常用命令

```powershell
pnpm test
pnpm build
pnpm tauri dev
pnpm tauri build
cargo test --manifest-path src-tauri/Cargo.toml
```

使用真实 PDF 语料执行只读导入测试：

```powershell
$env:PAPER_MASTER_TEST_CORPUS = "E:\path\to\papers"
cargo test --manifest-path src-tauri/Cargo.toml imports_configured_real_corpus -- --nocapture
```

测试使用内存数据库，不修改源 PDF。

## 生成新的迁移包

```powershell
pwsh -File .\scripts\New-TransferPackage.ps1
```

脚本在 `handoff/` 中生成带时间戳的 ZIP 和 SHA-256 文件。默认包含源码、文档、Git 历史和 `paper/` 测试语料，但排除所有可重建缓存、构建产物、本地数据库、日志、环境文件、发布可执行文件和 LM Studio 外部文件。若不需要 PDF 语料，可添加 `-WithoutPaperCorpus`。

## 开发文档

- [CODEX_HANDOFF.md](CODEX_HANDOFF.md)：跨电脑接管快照、已知问题和下一步；
- [AGENTS.md](AGENTS.md)：Codex 修改本仓库时必须遵守的约束；
- [ARCHITECTURE.md](ARCHITECTURE.md)：架构边界与扩展点；
- [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md)：阶段状态和完成标准；
- [VALIDATION.md](VALIDATION.md)：验证记录；
- [DEBUGGING.md](DEBUGGING.md)：日志、错误码和 LM Studio 排错；
- [CHANGELOG.md](CHANGELOG.md)：版本变更记录。
