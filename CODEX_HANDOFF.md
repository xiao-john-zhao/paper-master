# v0.4.4 接管要点（2026-10-27）

修复导航即卸载造成输入丢失及ResearchPage cleanup取消检索。KeepAliveView首次访问挂载，以hidden切换页面，之后直到App退出才销毁；覆盖library/settings/diagnostics/research/graph和comparison。CSS确保hidden不占网格，对比可见wrapper使用display:contents，避免固定modal新增网格行。不要恢复为view条件渲染页面，也不要将隐藏等同取消任务。

ResearchPage active仅控制参考论文刷新；任务清理仍在真正卸载时执行。Settings初次读取和busy时禁用fieldset防止迟到响应覆盖新输入。Diagnostics返回时刷新快照。ComparisonDialog“×”/遮罩只收起，明确取消按钮仍停止任务；Sidebar提供返回入口。新requestId重置旧编辑草稿，保存结果按requestId校验，不允许旧请求覆盖新结果；取消后须保留可读状态。

草稿是本次运行内存状态，不写浏览器存储、数据库、缓存或日志；不宣称退出后自动恢复未保存输入。现有SQLite队列重启恢复未改变。App允许注入ApplicationServices以执行真实导航树回归；默认仍使用单一应用服务实例。

版本0.4.4，下一维护目标0.4.5。新回归测试navigationState.test.tsx在StrictMode下操作实际App；将KeepAliveView改回inactive卸载时，原5项检查全部失败，恢复修复后通过。最终验证及发布状态见VALIDATION.md。

# v0.4.3 接管要点（2026-10-27）

发布范围：明确 AI 全字段编辑入口、锚定矢量缩放及局部图谱、独立 OpenAI 兼容 API、带语义基准和全文分段回退的本地分析、中文安装包及私有 GitHub 分发。

AI staged.rs 执行“有原文引文的整体语义基准 → 最高参数直接分析 → 降推理/降输入预算直接分析 → 全文重叠分段 → 分层综合 → 有界原文语义复核”。只能在 LM Studio 且 adaptive_analysis 开启时使用，避免默增在线调用费用。基准非真值，原文优先；supported 仅是一致性复核，不是正确性概率。分段最大256段/综合16层，不能省略失败段或默截原文；无通过的最终结果不得 append。取消覆盖全部阶段。

成功参数与 ReadingReport 在现有 settings 表按 analysis-profile:<version_id> 保存，和新分析版本/当前投影在同一事务提交；没有数据库结构变更，无需新迁移。library cache 格式2读1/2，目录仍v1；恢复包括报告及人工修订。人工编辑后的显示必须说明模型复核针对生成原文。

外部 API provider=openai，单独 api_base_url/api_model/api_key/api_response_format/api_reasoning_supported；Chat Completions 复用本地中文、结构和证据校验；在线门控在实际 transport 生效。LM Studio 本机地址规则不能放宽。队列快照不得绕过当前已撤销的联网授权。

GitHub 私有仓库 xiao-john-zhao/paper-master。普通用户下载 Release 的 NSIS 或 Windows ZIP；源码迁移脚本继续单独使用，排除 program file/data/logs 等个人目录。LM Studio 为可选外部运行时，分享包仅附联网安装脚本，不带程序或模型。

版本源已决定为0.4.3；下一目标0.4.4。最终基线见 VALIDATION.md，中文用户指南见 docs/下载安装使用指南.md。不要把下面的历史版本说明当作现行发布状态。

# v0.4.2 接管要点（2026-10-25）

当前版本 v0.4.2，下一目标 v0.4.3。用户本轮要求模型/时间标注、缓存可更换搬运、纠正伪完成分析、多关键词复选与固定字号图谱缩放；追加要求修改完成后用 LM Studio 本地补齐分析。

迁移 10 只清理未编辑 extractive 版本的系统占位，修复结论标题并回填可靠摘录，将 active extractive 标成 not_analyzed/ai_analyzed=0；读取时仍保留可编辑摘录，不损伤模型与人工结果。不要再把 paper.analysis.is_some() 等同于真实模型完成。

缓存目录由数据库旁的 cache-location.json 持久定位，默认位置保持兼容。搬运复制校验后才切换指针，原目录不删除；旧版默认缓存删除标记不能被自定义缓存复活。批量队列将配置保存在 settings 的 analysis-queue-settings:<paper_id>，执行时读取该快照，保证本地批次不会因交互服务切换改为在线调用。

关键词 tags[] 为多选交集，保留旧 tag 查询兼容；Sidebar 词表来自完整库。graphPoint 只投影坐标，图谱 SVG 外层不再整体 scale，文字和点击区域独立。完整记录见 docs/2026-10-25-v0.4.2-library-controls.md。下文为历史交接信息。

# v0.4.1 接管要点（2026-10-25）

当前版本 v0.4.1，下一目标 v0.4.2。缓存目录固定为 `%APPDATA%/app.papermaster.desktop/library-cache/v1/`，与发布版本号无关。迁移 9 使用 SQLite 触发器维护 `library_cache_pending`；所有 Database 连接的作用域结束后自动同步已提交变化。失败不能清空待同步队列或隐藏数据库内容。

启动先同步旧版未导出的写入/删除，再逐篇补回缺失记录，最后只为缺少缓存的论文初始化文件。数据库中已有字段、版本修订和选择优先。缓存保留结果原文、人工修订、元数据编辑标记、提取文字及删除标记；不要将 API 设置、凭据、完整提示词或 reasoning 写入缓存或日志。新文件格式必须演进 `cache_schema_version` 并保留旧版读取；不可用应用版本号命名个人数据目录。

本机维护入口 `application/paper-master.exe --sync-cache` 先通过 SQLite VACUUM INTO 备份，再生成缓存。GUI 的 Diagnostics → 论文文件缓存支持同步/恢复/打开文件夹；论文详情保留编辑接口。历史 EXE 继续读共享数据库，但不具备新 JSON 恢复代码。详见 [v0.4.1 记录](docs/2026-10-25-v0.4.1-persistent-cache.md)。下文为历史接管记录。

# v0.4.0 接管要点（2026-10-24）

本轮在 `feature/unified-library-v0.4.0` 实施，完成后本地 fast-forward 更新 `main`，发布标签 `v0.4.0`。原有改动已保存于 `03a6755`。当前统一启动入口 `application/paper-master.exe`；联网开关复用 privacy_mode，关闭时 Codex 配置回退本机 LM Studio，所有外部入口检查已保存开关。迁移 7 新增多结果版本表和当前选择，迁移 8 清理自动提取的 IEEE 会员称谓并保留人工元数据；禁止将编辑后的当前投影当成唯一模型结果覆盖其他记录。

439 个真实 PDF 已写入实际本机库；不移动源文件。143 条复核记录及逐文件索引在忽略目录 `data/validation/`，不要提交它们。实际模型样本已生成 Codex 版本。初始 LM Studio 长请求超时，重载并关闭 MTP 后最小普通/结构化请求恢复，完整本地两论文链路本轮未通过。完整状态见 [本轮记录](docs/2026-10-24-v0.4.0-unified-archive.md)。

# Codex 跨电脑接管说明

## 进度、排版与关系图（2026-10-23 后续增强）

新增任务进度服务、原生事件与全局任务中心，覆盖导入/自动与手动分析/对比/检索/下载/设置/绘图/导出。总量未知必须保留不定进度，不伪造模型 token 百分比。导入已转入 blocking worker，并有重复/回退计数回归测试。关系图模块分为 domain 的统计/布局、service 的读取/导出和 page 的交互；只使用本地已有标签及署名，不能将边解释为引文或确定同组。新增 `save_graph_image` Rust 命令和本地 blob 图片 CSP 支持；导出仅允许 PNG/SVG。

字号和留白已调整。前端 20 项、后端 60 项测试通过（7 项外部实时测试仍 ignored），生产构建和 NSIS 打包通过；最新基线见 VALIDATION.md 顶部。MSVC `build.rs` 的 Common Controls v6 manifest dependency 用于解决引入 native save dialog 后的测试程序启动异常，不应移除。浏览器预览的默认内存设置需使用 Mock，避免初始化后误调用桌面 IPC。生成产物仍使用 `application/paper-master-updated.exe`，版本源保持 0.3.1。

## PDF、Codex 与文献信息增强（2026-10-23）

保留全部已有工作树改动。新增后端 PDF 打开/重新关联、Codex provider、IEEE 检索/公开下载、无 IPC 的私有 WebVPN 窗口、作者信息和关联论文、摘要迁移 6。默认仍为本地分析，Codex 功能需 Settings 中明确启用联网授权。最新测试结果见 VALIDATION.md；学校订阅无人值守下载未通过完整端到端验证，必须保留“试验”标识及“问题清单”说明。真实账号密码只在用户 LOCALAPPDATA 的独立私有 TXT，不进入源码、日志、提示词或迁移包。版本源仍为 0.3.1。

用户进一步要求自然语言研究意图检索：现有检索是“需求理解 → 联网检索”两阶段，支持参考论文、目标作者/潜在课题组、主题/方法相似点和年份条件；缺失参照先澄清。结果 Schema 为 v2，严格执行关系/年份过滤，重新计算共同署名，排除参考论文自身和仅主题相似的同组替代。真实中文同组请求与缺失指代场景已通过联网测试。Codex 严格响应 Schema 必须把可空字段也列入 required，不能删掉此兼容修复。

## 本机接管与功能增强（2026-10-22）

当前工作目录为 `D:\paper-master\paper-master`。原交接文档改动和 Git 历史已保留，迁移 ZIP 的 SHA-256 已核对。Node、pnpm、Rust MSVC、VS C++ 工具链与 WebView2 已恢复并验证；本地构建/启动方式见 `LOCAL_DEPLOYMENT.md`。

本轮已按用户要求实现首次导入自动分析队列，以及使用有界原文证据的本地模型深度比较。原始版本号仍为 `v0.3.1`，新增行为归入尚未打标签的 `v0.3.2` 工作树。自动任务、比较语义/引文校验和当前 LM Studio API 兼容性是后续维护重点。最新实际验证结果请优先读取 `VALIDATION.md` 顶部。

下面的 2026-08-24 快照作为历史迁移背景保留，其中“下一版本尚无需求”“比较仅发送已保存摘要”等描述已被本轮明确需求与实现更新。

## 快照

- 生成日期：2026-08-24（Asia/Shanghai）
- 项目：`paper-master`
- 当前稳定版本：`v0.3.1`
- 下一目标版本：`v0.3.2`
- 分支：`main`
- 稳定基线提交：`c192565`（tag `v0.3.1`）
- 阶段：维护与缺陷修复
- Git 远端：未配置

在本次交接整理开始前，工作树为干净状态。本次工作新增或更新 README、Codex 指令、交接说明、验证记录、忽略规则和迁移脚本；这些文档类变更未自动提交。迁移包会保留 `.git` 历史和当前工作树，因此新电脑解压后应先运行 `git status --short --branch`，确认这些预期变更后再决定提交方式。

## 项目目标与当前状态

目标是提供一个 Windows 本地优先的论文资料库：从本地 PDF 建立可搜索的 SQLite 论文库，并可选使用本机 LM Studio 模型执行中文结构化分析、受证据约束的标签和多论文比较。

`v0.3.1` 已完成 PDF 导入、SQLite/FTS5、搜索与标签、Mock/LM Studio AI、2–8 篇比较、诊断链路、Windows 打包和论文出版物元数据修复。九个开发阶段均标记为完成，当前没有源码中的 TODO/FIXME 待办；下一版本尚未定义具体功能范围，应先由用户确认需求。

## 已确认的关键决定

- 本地优先：PDF 原地引用，不移动或删除源文件。
- Rust 基础设施负责 SQLite、文件解析、LM Studio HTTP 和持久诊断；React 只通过应用服务和 Tauri 命令访问这些能力。
- 数据库使用有序迁移，禁止以重建数据库替代兼容迁移。
- LM Studio 适配器只接受回环地址，并忽略系统代理；完整论文、完整提示词、响应正文和 reasoning 不进入日志。
- Mock provider 是无 LM Studio 环境的默认开发/故障路径工具。
- AI 论文分析使用版本化结构和严格 Schema；叙述字段必须满足简体中文检查，标签必须能在当前论文内容中找到证据。
- 多论文比较发送已保存分析与有界原文摘录，不发送无上限的完整提取文本；由模型进行语义比较并校验原文引文。

## 迁移包内容

默认迁移包包含：

- 当前工作树的源码、锁文件、配置、测试、文档和图标；
- `.git` 历史，使新电脑可直接查看提交与标签；
- `paper/` 中 12 篇、约 58 MB 的只读真实 PDF 测试语料；
- ZIP 内部 `_TRANSFER_MANIFEST.txt`，以及 ZIP 旁的 SHA-256 校验文件。

默认排除：

- `src-tauri/target`（约 17 GB）、其他 `target`、`node_modules`、`.pnpm-store`、`.cargo-home`、`dist` 和 `src-tauri/gen`；
- `.env`、数据库、应用日志和临时日志；
- `application/*.exe` 与 `application/*.lnk`，因为可从源码重建；
- `handoff/` 以前生成的迁移包；
- LM Studio 程序、模型、缓存、日志和旧电脑设置。

特别说明：`src-tauri/src/ai/lm_studio.rs` 等适配器源码属于可编译项目的一部分，因此保留在包内；它们不是 LM Studio 的程序或模型文件，也不会让新电脑自动安装或启动 LM Studio。

## 迁移前验证基线

当前机器工具链参考：

- Codex 随附 Node.js：`v24.19.0`
- Codex 随附 pnpm：`11.19.0`；项目声明版本为 `11.16.0`
- Rust/Cargo：`1.97.1`

2026-08-24 的后端基线：

```text
cargo test --manifest-path src-tauri/Cargo.toml
43 passed; 0 failed; 2 ignored
```

两个 ignored 测试均要求运行中的本机 LM Studio 服务，不属于默认基线。测试期间只有未使用字段/枚举变体的编译警告，没有失败。

旧电脑根目录的 `node_modules` 已损坏，缺少 `@babel/parser` 的 `package.json`，所以直接在旧依赖目录运行 `pnpm test` 和 `pnpm build` 会在启动前失败。这是本机依赖目录问题，不是源码测试失败；`node_modules` 不进入迁移包。新电脑必须执行干净的 `pnpm install --frozen-lockfile`，再运行前端测试和构建。

随后从提交快照建立独立临时副本，按 `pnpm-lock.yaml` 干净安装依赖；`pnpm test` 的 3 个测试文件、11 个测试全部通过，`pnpm build` 的 TypeScript 与 Vite 生产构建也通过。第一次安装时 Codex 随附的 Node 未在 shell 的 `PATH` 中，显式加入后安装成功；正常安装 Node.js 的新电脑应已具备该路径。完整结果记录在 `VALIDATION.md`。

## 已知风险与未完成事项

1. LM Studio 实时推理依赖外部软件、模型兼容性、内存/显存和本机服务状态；本次迁移不打包也不重新验证这些外部内容。
2. 应用数据库和个人导入记录不在项目迁移包中。若用户需要保留实际库状态，应另行备份 `app.papermaster.desktop` 的 Tauri 应用数据目录；PDF 路径跨电脑变化后可能需要重新导入。
3. `paper/` 测试语料不受 Git 管理。默认迁移包会携带它，但使用 `-WithoutPaperCorpus` 生成的精简包不会包含。
4. 旧的 `application/` 发布可执行文件不进入迁移包。需要发布参考时，在新电脑通过 `pnpm tauri build` 重新生成。
5. `VALIDATION.md` 中较早版本的历史失败记录是当时快照，不代表当前 `v0.3.1` 状态；优先读取顶部最新记录。

## 新电脑接管步骤

```powershell
# 1. 解压后进入项目
cd D:\dev\paper-master

# 2. 确认历史、标签和交接改动
git status --short --branch
git log --oneline --decorate -8

# 3. 使用锁文件安装全新依赖
pnpm install --frozen-lockfile

# 4. 建立新电脑基线
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml

# 5. 无 LM Studio 启动开发版，使用 Mock provider
pnpm tauri dev
```

如果只做前端工作，可先运行 `pnpm dev`。如果 Rust 链接失败，优先检查 MSVC Rust target 和 Visual Studio C++ Build Tools，而不是修改源码。

## 建议给新电脑 Codex 的首条指令

```text
请接管 paper-master。先阅读 AGENTS.md、README.md、CODEX_HANDOFF.md、ARCHITECTURE.md 和 VALIDATION.md，运行 git status 确认迁移文档改动，然后用锁文件安装依赖并执行前后端基线验证。不要安装或打包 LM Studio；无本地模型时使用 Mock provider。先报告环境差异和测试结果，再开始功能修改。
```

## 下一步

完成新电脑基线后，先让用户确定 `v0.3.2` 的具体目标。任何实现都应继续遵守 `AGENTS.md` 的隐私、本地优先、数据库迁移和只读 PDF 约束。
