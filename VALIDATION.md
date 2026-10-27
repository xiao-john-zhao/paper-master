# v0.4.5 完整应用分发验证（2026-10-27）

本版仅整理可运行应用与分发入口，未改变页面、AI或数据库逻辑。`scripts/Build-Local.ps1`先后完成前端35项测试（9文件）、后端93项测试（7项实时外部测试ignored）、TypeScript/Vite生产构建（153模块）、Rust release编译及NSIS安装器；测试和构建阶段均成功。脚本最终退出码1，发生于第33行Copy-Item覆盖本机主入口：旧应用正在运行，EXE被占用，并非产品编译回归。

恢复：将运行中的旧EXE移动到忽略的handoff/release-backups，保留旧进程，复制已构建的新EXE到正式入口；未停止任何用户进程。Update-ApplicationManifest.ps1退出码0并确认版本0.4.5、文件大小19542528、SHA256 `f08371db67b232199494edd4733e1f2cfd36c5e1f8e3180739ca070bcbdccbab`。Build-Local已加入同样的文件锁备份替换处理。脚本经PowerShell语法解析检查。

`New-ReleasePackage.ps1`退出码0，使用全新GUID暂存目录和显式文件列表。安装器分享ZIP仅含5个预期文件；免安装ZIP仅含9个预期文件（当前EXE、启动器、清单/校验及说明）。两种ZIP CRC通过，免安装包程序与正式入口逐字节一致，发布清单版本/大小/摘要一致，根校验文件的每个条目均通过。

负例检查：预先在旧的handoff/release-v0.4.5/windows-x64加入模拟缓存DO_NOT_PUBLISH_TEST_CACHE.db，再执行打包；两个ZIP均不含该文件或其他旧payload内容。没有使用真实个人缓存做试验。免安装ZIP大小7027248字节。

Git允许路径检查通过：application只允许paper-master.exe、release.json、SHA256SUMS.txt及README.md。历史EXE、随意JSON/DB、program file、library-cache、data、node_modules、target、dist、handoff、temp、临时文件、备份、环境文件和Python缓存的模拟路径均被忽略。待提交源文件常见令牌/数据库/缓存JSON结构扫描无命中。当前EXE是显式批准的可运行产物，不是本地缓存，不使用Git LFS占位。

使用git write-tree与git archive生成当前完整源归档，CRC通过；归档包含启动paper-master.cmd、快速开始.txt、版本清单及完整EXE，程序大小/摘要与清单一致，不是LFS指针。源归档仅含这一份EXE，没有缓存、数据库、临时文件或构建目录。v0.4.5标签沿用同一完整树。README、application说明和中文指南均区分免安装、安装器及Code ZIP；直接运行需要系统WebView2，完整安装器含离线组件。源码与发布包不包含个人数据库、API密钥、模型、登录状态或论文。

未在另一台全新Windows电脑重新安装/启动，也未运行真实外部模型。旧进程继续使用其原版本，退出后从新版入口打开才生效；本次没有为了验证启动再开一个真实资料库进程。

# v0.4.4 页面切换与任务状态修复验证（2026-10-27）

最终命令 `pwsh -File scripts/Build-Local.ps1` 退出码0：前端35项通过（9文件），后端93项通过、7项实时外部测试ignored，TypeScript/Vite生产构建、Rust release及中文NSIS打包通过，153个前端模块。`scripts/Check-Version.ps1`确认版本源/锁记录一致0.4.4。

新增6项StrictMode实际App树回归，使用隔离内存库与可控制的异步服务：

- 侧栏往返保留联网检索输入、篇数、参考论文、未保存API密钥、元数据/AI草稿及图谱缩放；未访问页面仍延迟挂载，返回不重复构建。
- 模拟联网检索在隐藏页面及blur/focus/visibilitychange后继续，同一requestId只调用一次；隐藏期间完成的结果返回可见，不触发cancel。
- 返回页面后明确取消检索仍生效，且不重复取消。
- 设置后台连接检查完成后保留结果，初次读取和执行中表单禁用，避免覆盖草稿；未点击保存不写入设置。
- 进行中对比收起/返回不取消、不重发；隐藏时完成结果仍可查看，对比编辑草稿保留。
- 对比只有明确点击取消才停止，返回保留可读的已取消状态，并阻止旧请求覆盖新结果。

故障复现：临时将KeepAliveView恢复为inactive即卸载的旧生命周期，`node node_modules/vitest/vitest.mjs run tests/navigationState.test.tsx`退出码1，原5项检查全部失败（输入DOM重建、检索被cancel、连接结果和对比视图丢失）。通过finally恢复修复文件后，同命令5项全部通过；补充第6项取消回归后完整35项通过。故障注入日志保留在忽略目录handoff/validation/。首次打印注入摘要时Python默认GBK不能输出Vitest符号，打印脚本发生UnicodeEncodeError；完整测试日志已先写入，源码已由finally恢复，后续测试通过。早期`git diff --check`发现App.tsx行尾空格，清理后退出码0。

`New-ReleasePackage.ps1`退出码0；ZIP CRC与内容白名单通过，只有安装器、中文指南、可选LM Studio脚本和校验文件，内外安装器SHA256相同。本机application/paper-master.exe与构建产物摘要一致。安装器字节数222528099，SHA256 `67ccc5a4eff7bdcb835393ccd141a269b4f0d093fb75fedbe86ed92443251e51`。

本次没有改动数据库模式、AI协议、分析参数或持久队列，不需迁移。待提交路径与常见令牌扫描均通过，未包含个人论文、数据库、密钥、安装器或缓存。草稿只驻留App会话内存，退出/崩溃/重载不保证恢复未保存输入；持久分析队列重启恢复保持原行为。没有运行真实外部模型、付费检索或新电脑人工安装测试；DOM生命周期回归不等同于桌面视觉验收。

# v0.4.3 发布验证（2026-10-27）

最终命令：`pwsh -File scripts/Build-Local.ps1`，退出码0。按脚本顺序完成锁文件依赖安装、前端测试、后端测试、TypeScript/Vite生产构建、Rust release编译、NSIS打包和本机应用替换。`scripts/Check-Version.ps1`退出码0，五处版本源/锁记录一致为0.4.3。

- 前端：8个文件、29项通过。包含九类AI字段编辑保存/恢复、修订状态、锚定缩放坐标不缩放字形、节点局部筛选/聚焦/重置。
- 后端：93项通过、0失败、7项实时外部测试ignored。新增Bearer/自定义服务前缀/JSON模式传输、联网门控、逐篇参数重置、失败分级重建材料、原文引文校验、全分段覆盖/分层综合/语义复核、复核冲突不替换旧版本、取消及报告缓存恢复。
- 分段故障注入：先记录baseline，模拟direct失败，再覆盖约8万字符原文；最后的限制条件在多轮综合后仍保留。冲突复核拒绝新版本，旧选择继续保留。测试使用隔离内存库和模拟服务，不是实时LM Studio推理。
- Windows x64中文NSIS构建通过，含WebView2离线运行时。安装器222470375字节，SHA256 `e95496d2c7585057d8b715017b69d9c3a9362169ddee0c1d630d42193bebdfa7`。
- `scripts/New-ReleasePackage.ps1`退出码0。ZIP CRC通过，内容严格为安装器、中文指南、可选LM Studio脚本、SHA256SUMS；内外安装器SHA256一致。本机application主入口与构建EXE的SHA256一致。
- 待提交文件列表未包含PDF、数据库、EXE、环境文件、凭据TXT、program file或模型；已对源码做常见令牌模式扫描，无命中。Git历史对象路径未发现上述个人文件。分享ZIP不包含Git历史或个人数据。

本轮实际修复的验证失败：首次`pnpm build`退出码1，AIProvider.id未扩展openai；补齐统一类型后通过。新编辑测试首次`pnpm test`退出码1，保存后使用旧paper对象导致preview版本修订状态被覆盖；改为从repository读取当前投影后通过。首次分段接线`cargo test --locked --manifest-path src-tauri/Cargo.toml`退出码101，进度usize未转u64；修正后通过。最终完整脚本无失败。

限制：没有重新运行真实LM Studio/付费API推理，没有在另一台全新Windows电脑安装验收。桌面交互自动验证因浏览器后端不可用/原生浏览器地址识别受限未完成，不能把DOM交互测试写成桌面人工验收。Windows二进制依赖检查仅见系统DLL及UCRT，未引入额外开发工具链依赖。语义复核为有界证据启发式，不是正确性证明、最优性证明或校准概率。分段中间状态退出后重跑，不保留为完成分析。当前编译仍有此前的dead_code/linker提示，无新增测试失败。

# v0.4.2 verification — 2026-10-25

- Frontend: 26 tests passed across 7 files, TypeScript/Vite production build passed (152 modules). Checks cover persistent multi-tag checkboxes/intersection, truthful model/time labels, and coordinate zoom with unchanged text size/node radii.
- Backend: 87 passed, 0 failed, 7 external live tests ignored. New checks cover cache relocation/cold recovery/manual moves/future-format rejection/legacy deletion tombstones, source-only conclusion repair/manual preservation, multi-tag canonical search, queued provider/model pinning, and bounded model-header extraction without retaining body diagnostics.
- Full-library isolated migration and actual migration: 439 papers / 885 historical versions retained; paper metadata and 7 protected model/manual versions match pre-migration digests. Schema 10, integrity ok, cache pending 0. Synthetic unedited active placeholders cleared; 399 extracted conclusions recovered; current model-complete rows corrected from 439 to 6 and 433 excerpts marked pending analysis.
- Native desktop: two keyword checkboxes selected simultaneously, whole facet list/counts remain; actual graph 100%→120% spreads coordinates with stable text/node sizes; cache-directory picker opens and cancels correctly; model/time labels, Production 0.4.2 and schema 10 visible.
- Explicitly authorized local analysis: LM Studio server/model qwen/qwen3.8-27b reused on 127.0.0.1:1234. One real paper completed the full production analysis/Chinese/Schema/evidence/tag/cache pipeline, with 7 stored tags and an independent model version; no archive placeholders remain in that output. Detailed private report: data/validation/v0.4.2-local-sample-result.json.
- Remaining local batch started: 432 queued, 7 already complete skipped, 0 without text. Actual worker snapshot: 7 analyzed / 1 analyzing / 431 queued; 432 job configuration snapshots pinned to lmstudio + qwen/qwen3.8-27b + local_only. Cached outbox 0, integrity ok. These are running tasks, not completed results; completion/failure counts must be read from the live queue. Closing the GUI interrupts the current task and restart resumes it.
- Release EXE + NSIS x64 installer built; version sources and Cargo.lock aligned to 0.4.2. Source PDFs and all original protected results remain unchanged. Personal snapshots, cache files, model/runtime data and diagnostic details remain excluded from Git.

# v0.4.1 verification — 2026-10-25

- `pnpm test`: 22 passed. `pnpm build` and the production Tauri before-build frontend step passed with 151 modules.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml`: 78 passed, 0 failed, 7 external live tests ignored. Eight new tests cover cache recovery and the backup/maintenance path; the default suite does not call external models.
- Real library: 439 papers, 882 saved result versions. All paper rows, version rows and active selections match the pre-migration logical SHA-256 digests. Schema upgraded from 8 to 9, integrity `ok`, cache pending 0, unreadable 0. Cache files are kept under the stable APPDATA directory, outside the repository.
- Full cache-only recovery: copied only JSON cache to an isolated profile with no database, then ran the production `--sync-cache` command from the Windows directory. Recovered 439 papers + 882 versions (1,321 records), with exact original paper/version/selection digests; integrity `ok`, foreign_key_check empty, pending 0, unreadable 0. Private reports: `data/validation/v0.4.1-cache-check.json` and `v0.4.1-cache-only-recovery.json`.
- Release EXE and NSIS x64 installer built successfully. Unified and versioned EXE are under application/. Version source / Cargo.lock checks, Rust formatting and Git whitespace checks pass. The original v0.4.0 EXE is preserved under handoff/release-backups/.
- First actual maintenance run unexpectedly read an empty database after closing the backup connection. The complete pre-migration SQLite backup retained every paper/version; it was restored immediately, with exact logical digests confirmed. This did not reproduce on the isolated populated copy or subsequent actual runs. The maintenance path now reuses one SQLite connection through backup/migrations/export, refuses implicit recreation of an existing database, and checks before/after migration counts. No specific external/WAL cause is claimed. All backup and diagnostic files remain private.
- Browser preview automation could not discover a Codex IAB backend. Native Computer Use confirmed the actual 439-paper library and editing controls; a release-only hidden Diagnostics navigation entry was found and made always visible so cache controls are accessible. Final native checks: Production 0.4.1 / schema 9 / 439 papers, Diagnostics cache controls visible, sync success message, recovery adds 0 duplicates, metadata-edit button count remains 1 after successive paper switches. Final live library has 883 result versions (one new version appeared during acceptance); all 883 are cached with 0 pending and integrity ok.

# v0.4.0 verification — 2026-10-24

- Frontend: 21 tests passed; TypeScript/Vite production build passed (149 modules).
- Backend: 70 passed, 0 failed, 7 ignored; migration 7/8 preservation, IEEE role cleanup/manual metadata protection, versions/edit conflicts, team inference and persisted network gate covered.
- Full corpus: 439 PDF imports into memory, 439 duplicates on reimport, integrity ok; then actual library archive 436 new + 3 existing, 0 failures. All 439 source SHA-256 values unchanged, foreign_key_check empty.
- Quality review: 107 missing authors, 32 unmatched keyword records, 18 missing abstracts, 143 unique review records; 722 potential-team links. Full local reports remain under ignored data/validation/.
- Real Codex sample persisted independently alongside existing results; no summary in diagnostic logs. Qwen initial 600-second timeout was not hidden; after reload without Draft MTP, minimal ordinary completion passed in 2.05 seconds and the structured smoke passed in 63.82 seconds. The initial full local two-paper pipeline remains unverified this round.
- Release + NSIS build passed. See docs/2026-10-24-v0.4.0-unified-archive.md for commands, boundaries and the initial environment failures.


# Progress, typography and relationship graph verification (2026-10-23)

- `pnpm test`: 20 passed across 5 files. New checks cover co-occurrence deduplication, evidence paper IDs, shared-name/keyword edges and filters, graph size limits, deterministic finite layout, escaped full export labels, independent task lifecycles and late-event rejection after cancellation.
- `pnpm build`: TypeScript and Vite production build passed, 146 modules. `cargo test --locked --manifest-path src-tauri/Cargo.toml`: 60 passed, 0 failed, 7 external live tests ignored. New Rust checks cover file progress with duplicate/fallback/missing paths, unchanged synthetic source PDFs and image payload validation. `cargo fmt -- --check` and Git whitespace checks passed (Git reports existing Windows line-ending conversion notices).
- Configured read-only corpus test passed in 18.34 seconds using an in-memory database: 12 imported, 0 failures, 2 isolated parser fallbacks, 12 duplicates on reimport, 10 FTS matches, successful Chinese-path import. All 12 source PDF SHA-256 values were identical before/after this run.
- Local browser UI verification: analysis transitioned from indeterminate running to completed; graph and export showed actual stages; keyword node/edge inspection showed the correct supporting paper and opened its detail; paper mode exposed 4 paper nodes and the author-link toggle. Checked narrow (950 px) and default desktop (1440 × 900) layouts. Saved screenshot: `.handoff-verify/relationship-ui.png` (demo metadata only).
- Actual browser exports produced a valid 1600 × 1350 PNG (162,234 bytes) and an XML-parsable SVG with the full labels/legend. Export scaling is uniform. Native export uses a backend save dialog and PNG/SVG-only validation; the native dialog interaction itself has not been automated. Native CSP allows local blob images for rasterization.
- Initial Rust test startup exited `0xc0000139` (`STATUS_ENTRYPOINT_NOT_FOUND`). Import inspection identified `TaskDialogIndirect` without Common Controls v6 selection after linking the save-dialog path. An initial `/MANIFEST:EMBED` attempt caused duplicate manifest resource `CVT1100/LNK1123`; retaining only `/MANIFESTDEPENDENCY` in MSVC `build.rs` fixed both harnesses. The full test suite then passed. Release manifest extraction also confirmed Common Controls 6. See [Microsoft's Common Controls manifest guidance](https://learn.microsoft.com/windows/win32/controls/cookbook-overview).
- Final `pnpm tauri build --ci -- --locked` passed and produced the portable release and NSIS x64 installer. The final executable is copied to `application/paper-master-updated.exe`; version sources stay at 0.3.1. Existing user data and private credential storage are unchanged. The prior WebVPN authenticated-download limitation below remains in effect.

---

# Local takeover and enhancement verification (2026-10-22)

The original v0.3.1 version sources and both lockfiles remain unchanged. New behavior is in the unreleased v0.3.2 worktree. Original handoff changes and Git history were preserved.

## Environment and deterministic checks

- Windows local deployment: Node 24.14.0, pnpm 11.25.0, Rust/Cargo stable MSVC 1.98.0, Visual Studio 2022 C++ tools, Windows SDK and WebView2 available.
- Frontend: `pnpm test` — 13 passed across 3 files; `pnpm build` — TypeScript and Vite passed (139 modules).
- Backend: `cargo test --locked --manifest-path src-tauri/Cargo.toml` — 50 passed, 0 failed, 3 ignored. New coverage includes queued imports, opt-out, duplicate handling, interrupted-job recovery, failure isolation, queued-request cancellation, eight-paper total input budgets, invented/misattributed citation rejection and per-paper constrained citation schemas.
- The 12-PDF read-only corpus import smoke test passed using an in-memory database. All 12 source PDF SHA-256 values also match their entries in the original transfer ZIP.
- PowerShell build/start scripts pass parser checks; `Start-Local.ps1 -CheckOnly` connects to the local loaded model. `cargo fmt -- --check` and Git whitespace checks pass.

## Desktop and local runtime checks

- Production Tauri executable and NSIS installer generated locally. The launch script and desktop shortcut use the portable build at `application/paper-master-local.exe`.
- Actual native desktop smoke test: Settings can discover/load the existing Qwen3.8 27B Q4_K_M model; the persisted provider is LM Studio, automatic import analysis is enabled, reasoning is disabled through API `none`, and output budget is 4,096 tokens. Database integrity is `ok`.
- Actual native import of one corpus PDF automatically entered `analyzing` and completed with a persisted v2 Chinese analysis and supported tags in 204,746 ms, retry count 0. The source file remains unchanged.
- LM Studio API listens on 127.0.0.1:1234; CORS is disabled, sensitive body/token logging is disabled, and API auto-start on LM Studio launch is enabled. Original server settings were backed up outside the repository.
- Final runtime was reloaded with a 24,576-token context and `parallel=1`; the local startup script successfully reused it. The final executable was copied from the production build and SHA-256 checked: `6D09703DD3BEBC51288CC3FAC19E57E143F1A121587FFF10783AF647608D39D6`.
- Two initial environment/API failures were resolved: the current load API returns `instance_id`, and OpenAI-compatible disabled reasoning must be `none` rather than native-capability `off`.
- Real inference exposed unsupported generated tags and paraphrased quotations. Tag repair now receives source-only terminology candidates; citation output is constrained to literal per-paper passage choices and independently checked. A local API test accepted the constrained `oneOf` evidence schema and returned two valid evidence objects.

## Final real-model end-to-end result

`services::analysis_queue::tests::live_import_auto_analysis_and_evidence_comparison` passed against the existing local Qwen3.8 27B Q4_K_M model: two real corpus PDFs imported and queued, two Chinese analyses persisted to the in-memory test database, and a fresh model comparison returned 2 substantive differences and 2 source-attributed verbatim quotations. The same production import, queue, provider, comparison builder, response Schema and validators were exercised. Result: 1 passed, 0 failed, wall time 540.12 seconds. This smoke test used a 12,000-character total input budget and 4,096 output tokens; the separate native desktop import used the configured 24,000-character input budget.

No full paper, prompt, response body, or reasoning was written into application diagnostics. The test also asserts that a returned source quotation is absent from its diagnostic log. The source PDFs remain in their original locations and match the transfer archive byte for byte.

---

# v0.3.1 handoff validation (2026-08-24)

Validated while preparing the repository for transfer to another Windows development computer.

## Backend baseline

- `cargo test --manifest-path src-tauri/Cargo.toml` passed.
- Result: 43 passed, 0 failed, 2 ignored.
- Both ignored tests require a running local LM Studio server and a compatible local model; LM Studio external files were intentionally not included or exercised for this handoff.
- Compilation emitted only existing dead-code/linker warnings.

## Frontend dependency finding

The existing root `node_modules` was incomplete: pnpm reported a missing `@babel/parser/package.json` before Vitest or the TypeScript build could start. This is a local dependency-directory failure, not a source-test assertion failure. `node_modules` is excluded from the transfer package; the target computer must run `pnpm install --frozen-lockfile` before validation.

Clean-install frontend verification was performed from the committed snapshot in an isolated transfer-check directory so the damaged root dependency directory was not copied or repaired in place. Dependencies were restored from `pnpm-lock.yaml` with pnpm 11.19.0 and Node.js 24.19.0.

- `pnpm test` passed: 3 test files, 11 tests, 0 failures.
- `pnpm build` passed: TypeScript compilation and the Vite production build completed, with 139 modules transformed.
- The first install attempt in the Codex shell required adding Codex's bundled Node directory to `PATH` for the esbuild postinstall script. A normal Node.js installation on the target computer should already provide `node` on `PATH`.

## Transfer policy

- Include source, lockfiles, documentation, Git history, and the read-only local `paper/` test corpus.
- Exclude Rust/Node build outputs and caches, local databases/logs/environment files, release executables, and all external LM Studio programs, models, caches, logs, and settings.

---

# v0.3.0 validation

Validated on Windows against the left-library-preview changes and the production Tauri release build.

## Preview and metadata checks

- The left preview renders paper title, publication venue, and authors as its first three rows; file name and year are no longer used in the compact header.
- Keyword tags appear only for papers whose AI analysis status is `analyzed`.
- In-app browser verification covered both analyzed and not-yet-analyzed rows, including the presence/absence of tags.
- A real IEEE paper from the local `paper` corpus was used to reproduce its first-page running journal header and multiline title. Regression tests confirm the parser returns the joined paper title, the simplified journal name, and author names without IEEE membership labels.
- Database migration 5 preserves existing papers and backfills publication venue and corrected titles from locally stored extracted text.

## Automated checks

- Frontend: 10 tests passed; TypeScript/Vite production build passed.
- Scoped Rust metadata, migration, repository, and search tests passed.
- Tauri release compilation completed successfully for `paper-master` v0.3.0.
- The full Rust suite currently has 5 failures in pre-existing structured-output/prompt assertions outside this layout and metadata change; those unrelated modules were not modified for this fix.

## Release artifact

- `application/paper-master-v0.3.0.exe`

---

# v0.2 validation

Validated on Windows with the production Tauri build.

## Automated checks

- Frontend: 9 tests passed across schema, repository/search, analysis persistence, comparison preconditions, and three-paper comparison.
- Rust: 27 tests passed across migrations, Unicode path normalization, SQLite/FTS, PDF cleaning and section detection, local-only settings, native LM Studio model loading, Chinese AI output, evidence-constrained managed tags, cancellation, import fallbacks, and 2–8-paper request validation.
- TypeScript/Vite production build completed successfully.
- Tauri release compilation and NSIS x64 installer packaging completed successfully.

## Real PDF corpus

The recursive import smoke test used 12 local PDFs through the production import/parser/repository code path and an in-memory validation database. Source files were read-only.

- 12/12 paper records imported; 0 import failures.
- 2 structurally incompatible PDFs were isolated and saved as fallback records instead of aborting the batch.
- Re-import detected 12/12 duplicates.
- FTS query `communication` matched 10 papers.
- A real extractable PDF copied to a temporary Chinese directory with a Chinese filename imported successfully, preserving 50,768 extracted characters and a normal user-facing Windows path.

This corpus exposed two third-party parser panics (an object-index edge case and an unsupported `UniGB-UCS2-H` font encoding). The document parser boundary now catches these failures, records `PDF_READ_FAILED` diagnostics, and allows the remaining import to continue.

## Release smoke test

The versioned executable at `application/paper-master-v0.2.0.exe` started successfully, created/opened its Windows application-data database, migrated the schema to version 3, and appended startup/migration events to the JSONL log. The smoke-test process was then closed normally by the validation harness.

LM Studio was not running during final packaging; local model inference therefore remains an environment-dependent manual acceptance step. The deterministic Mock provider and structured-output parser paths are covered by automated tests.
# PDF / Codex / IEEE enhancement verification (2026-10-23)

- `pnpm test`: 13 passed across 3 files; `pnpm build` / the Tauri before-build step: TypeScript and Vite passed, 141 modules.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml`: 58 passed, 0 failed, 7 ignored. Added abstract-boundary and migration-preservation checks, corresponding-author footnote regression, Unicode/missing PDF path checks, online-mode consent validation, source URL checks, enforced relationship filters and nullable output-schema requirements.
- Natural-language live test `services::research::tests::live_natural_language_group_search -- --ignored --exact` passed in 129.04 seconds: interpreted a Chinese request for the reference paper's group and related UAV/ISAC research, returned source-supported author/team candidates, and asked for clarification when “这篇论文 / XXX” lacked a reference. An initial run exited 1 with `CODEX_UNAVAILABLE` because the intent's nullable year fields were not required in the output Schema; recursively requiring every property (while preserving nullable types) resolved the compatibility failure. The normal suite has an explicit regression check.
- Explicit live Codex checks passed: `ai::codex::tests::live_codex_reads_real_paper` exercises real read-only PDF import, the production analysis input builder/registry/commands, structured Chinese validation, SQLite persistence and privacy-safe application diagnostics (43.69 seconds); `services::research::tests::live_codex_searches_ieee` returned verified-format IEEE candidates (54.42 seconds). These use the current signed-in Codex account and are ignored in the normal suite.
- `services::research::tests::live_downloads_open_pdf -- --ignored --exact` passed: real HTTPS arXiv PDF retrieval and PDF-header validation, 2.83 seconds. Test-owned download removed after verification; corpus PDFs unchanged.
- Configured read-only real-corpus import test: 12 imported, 0 failed, 2 isolated parser fallbacks, 12 duplicates on reimport, 10 FTS matches and a successful Chinese-path import; 20.14 seconds. Existing third-party parser panics are caught per PDF and do not abort import. All 12 source PDFs also match their SHA-256 values from the original transfer ZIP.
- Actual desktop Open PDF launched WPS and rendered the selected 11-page corpus document. The application database upgraded to migration 6, integrity check returned `ok`, and all three existing abstracts exclude Index Terms and author/affiliation footnotes.
- Actual IEEE access through the supplied WebVPN root showed the university institutional-access banner and subscribed search results. The tested PDF stamp page remained blank; the new application's isolated browser profile requires its own login. Institution-authenticated download/import has NOT been verified end to end and remains explicitly experimental; limitations and fallback steps are in `问题清单/2026-10-23-联网检索与下载问题清单.md`.
- Final `pnpm tauri build --ci -- --locked` succeeded and produced a release executable plus the NSIS installer. Final executable is copied to `application/paper-master-updated.exe`; old running executable was preserved after the user stopped Computer Use. No further desktop interactions were performed after that interruption.
- School credential TXT is outside the repository in the current user's LOCALAPPDATA private folder. New Git/transfer exclusions cover credential text and browser-profile folders; no real credentials are embedded in application source or settings. LM Studio stays restricted to loopback; Codex requires explicit saved online consent. Version sources stay at 0.3.1.
