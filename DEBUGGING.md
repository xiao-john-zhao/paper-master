# v0.4.5 下载后如何打开

- Code ZIP/免安装ZIP：完整解压后双击根目录启动paper-master.cmd，或application/paper-master.exe；从v0.4.5起自带程序，无需编译。
- 安装器ZIP：先解压并运行_x64-setup.exe，然后从开始菜单打开。
- 缺WebView2：优先使用含离线组件的setup.exe；免安装包不会重复捆绑这个系统运行时。
- 找不到EXE：确认下载当前版本且完整解压，更早版本Source code ZIP可能只有源码。
- 校验位置：application/release.json / application/SHA256SUMS.txt；这些清单不包含个人设置。勿把个人缓存复制进应用目录或分享包。

# v0.4.4 页面切换与任务

- 运行期间侧栏切换会保留页面输入、进度、检索结果和图谱视野；返回设置不会重新读取并覆盖尚未保存的草稿。初次加载、连接检查或保存期间表单暂时不可编辑。
- 对比窗口“×”只收起。侧栏“查看进行中的对比 / 查看上次对比”可返回；只有明确点击取消才停止任务。取消后的状态仍可查看和重试。
- 未保存输入仅留在本次运行内存，退出前仍须点击保存。关闭应用、重新加载WebView或程序崩溃不是页面切换，不保证恢复这些草稿；既有持久分析队列继续按原规则在重启后恢复。
- 若复现页面切换清空，请先核对Diagnostics版本为0.4.4，避免运行旧版EXE。

# v0.4.3 常见问题

- 展开详情“处理方案与语义可信度复核”查看基准、最终一致性判断、引文及分段范围。基准不是裁判，原文优先；人工编辑后复核记录仅适用于生成原文。
- 高档直接分析失败后先降参，仍失败再覆盖全部提取文字分段阅读。段落失败、证据引文不匹配、综合整类丢失、覆盖缺口或复核不通过均不保存新完成版本。中断后会重跑整篇；不把半成品标记为成功。
- 自适应参数只影响当前篇，下一篇重新最高档；不会强制卸载已加载的模型。报告中的“请求加载上下文”不是已加载实例的实测值。
- 外部 API 必须启用联网并保存；地址含服务要求的前缀，不能携带查询密钥。无/models的服务可手填模型直接分析。遇到格式不兼容按服务文档切换JSON Object/Schema，不支持reasoning_effort则取消对应选项。
- 图谱缩放范围20%–1200%，滚轮锚定鼠标，文字/节点可独立调节；局部筛选仍只覆盖当前显示图，省略关系的数量见页面底部。
- 缓存格式2读取1/2，目录继续library-cache/v1；回退旧软件前使用升级前备份，不能假定旧软件识别新缓存格式。

# v0.4.2 常见问题

- 结果来源为“原文摘录（非模型）”：这不是模型分析。旧版本误标的状态由迁移 10 修复；详情显示待分析，可单篇分析或点击“补齐待分析”。真实模型输出与人工修订保留。
- “模型未记录”：历史版本没有保存具体型号，不能根据当前设置补造。新 LM Studio 结果保存实际模型，新 Codex 结果读取 CLI 头部的型号；总结时间和人工修改时间分别显示。
- 缓存换盘/搬运：Diagnostics → 更改位置 / 读取搬运的缓存。选择新目录会校验复制后切换，源目录保留；手动搬运后选择搬运后的 v1 文件夹即可。cache-location.json 在稳定应用数据目录，丢失数据库时仍能找到自定义缓存。
- 标签不消失：侧栏完整词表来自全库，勾选多个表示同时包含；清除所选恢复筛选。
- 图谱缩放：20%–500% 调整节点间距与视野，字号和节点大小固定。拖动空白处平移，重置恢复初始视野。
- 批量本地任务：任务入队固定模型，退出后重启续跑。每篇成功才算模型分析完成，失败保留原因及已有结果；源文件无可读文字的记录不发给模型。

# v0.4.1 论文结果恢复

先在桌面应用的 Diagnostics → 论文文件缓存查看固定目录、待同步数及错误。结果在论文详情编辑，保存后缓存自动更新。

- 打开不同版本/不同目录：同一用户保持 `app.papermaster.desktop` 应用标识，使用同一个 `%APPDATA%` 数据库；v0.4.1 起启动自动读取 `library-cache/v1` 补回缺失记录。
- 待同步数非零：检查空间/权限后点击“立即同步缓存”；成功保存的 SQLite 结果不会因缓存错误丢弃。
- 缺少记录：点击“从缓存补回缺失记录”。已有人工修改优先；用户删除的记录有删除标记，不自动恢复。
- 单份 JSON 损坏或缺失：尝试上一份 `.json.bak`，原损坏文件保留；未来格式不覆盖。备份也不可用时只跳过该篇并显示数量。
- SQLite 本身损坏：保留原库并单独处理，不静默重建。`--sync-cache` 在迁移前会创建本地数据库备份；不要把缓存目录当作临时文件清理。
- 只看到演示论文：浏览器预览使用内存演示库。请启动 `application/paper-master.exe`。

# v0.4.0 notes

- When Codex is selected but networking is disabled, analysis uses local LM Studio. Switch history in the paper result selector; re-analysis adds a version.
- Stale edits are rejected using a revision number; refresh before retrying. “恢复生成文字” restores the saved pre-edit baseline.
- Draft MTP plus the local Qwen runtime was present during a 600-second timeout. Reloading without Draft MTP and using a 16,384 context restored ordinary/structured smoke requests. Treat this as a tested recovery combination, not proof of a single root cause. Start-Local.ps1 -CheckOnly disables MTP on a new CLI load.
- Windows cannot relink a running test EXE. Finish or stop that test before rebuilding the same debug test target; release builds use a separate target.


# Debugging

## Development mode

Run `pnpm tauri dev`. Development builds include frontend source maps and the WebView inspector can be opened with `Ctrl+Shift+I` or the context-menu **Inspect** action. Set `RUST_BACKTRACE=1` before starting the app to include Rust backtraces in the terminal during development.

## Logs

The Diagnostics page reports the exact log path. Logs are JSON Lines and include timestamp, level, module, operation, error code, request ID, and duration where relevant. They intentionally exclude full paper text, file content, complete AI prompts, and model reasoning.

Use a request ID such as `analysis-YYYYMMDDHHMMSS-xxxxxxxx` to trace UI, provider transport, JSON validation, and database-save events for one operation.

The page can copy a compact diagnostic summary and open the JSONL log directory. Recent error events are held in memory for display; the durable log remains append-only.

## LM Studio OpenAI-compatible endpoints

paper-master treats the configured Base URL as the LM Studio server root or its OpenAI-compatible `/v1` root. These values are equivalent:

```text
http://127.0.0.1:1234
http://127.0.0.1:1234/
http://127.0.0.1:1234/v1
http://127.0.0.1:1234/v1/
```

The final OpenAI-compatible requests must always be:

```text
GET  /v1/models
POST /v1/chat/completions
```

LM Studio Server Logs are the quickest way to detect path loss. A paper-master analysis must show `POST /v1/chat/completions`. If the log instead shows `POST /chat/completions` together with `Unexpected endpoint or method`, the client has incorrectly replaced the `/v1` base path. HTTP 200 alone is not proof of success because LM Studio can return 200 for an invalid endpoint; paper-master also validates the JSON body and requires a non-empty `choices[0].message.content`. Invalid success bodies use `LMSTUDIO_INVALID_RESPONSE`.

The Diagnostics JSONL transport event records `request_id`, `provider`, `method`, `endpoint`, `model`, `http_status`, `finish_reason`, `content_present`, `content_length`, `reasoning_present`, `reasoning_length`, `tool_calls_present`, and `duration_ms`. It never records the full paper text, complete prompt, final response body, or reasoning text.

Paper analysis requests use LM Studio's OpenAI-compatible `response_format.type=json_schema` mode. The JSON Schema is generated from the Rust `PaperAnalysisResult` runtime type, rejects additional properties, and requires every v2 analysis field. `max_tokens` comes from the persisted **Maximum AI output** setting (default: 8,192) so reasoning models retain bounded room for both reasoning and the final structured response.

### AI response extraction

Use the `stage` field to locate the exact stopping point for an analysis request:

```text
PDF_INPUT_READY
REQUEST_BUILT
REQUEST_SENT
RESPONSE_RECEIVED
FINAL_CONTENT_EXTRACTED
JSON_PARSED
SCHEMA_VALIDATED
DATABASE_SAVED
ANALYSIS_COMPLETED
```

An HTTP 200 response with empty `choices[0].message.content` stops at `FINAL_CONTENT_EXTRACTED` with `AI_FINAL_CONTENT_EMPTY`. Separate `reasoning_content` or `reasoning` is never used as the final answer; only its presence and character count are logged. `finish_reason=length` uses `AI_OUTPUT_TOKEN_LIMIT`. Syntactically invalid JSON uses `AI_INVALID_JSON`, while valid JSON that violates the generated schema uses `AI_SCHEMA_VALIDATION_FAILED`.

The development-only minimal compatibility smoke test uses fictional metadata rather than a real paper:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml ai::lm_studio::tests::live_minimal_structured_output_smoke -- --ignored --exact --nocapture
```

Set `PAPER_MASTER_LMSTUDIO_URL` or `PAPER_MASTER_LMSTUDIO_MODEL` to override the default local endpoint or `deepseek-r1-distill-llama-8b`. `PAPER_MASTER_LMSTUDIO_TIMEOUT_MS` can extend the read-only real-paper smoke test without changing the saved application setting. If the model reasons but returns no final content, the minimal check reports `MODEL_STRUCTURED_OUTPUT_INCOMPATIBLE`; it never parses the reasoning text.

## Subsystem checks

- Automatic imports: new text-bearing PDFs enter `queued`, then `analyzing`, then `analyzed` or `failed`. Original PDFs stay in place. The backend runs one inference at a time; cancelled requests waiting for inference also exit promptly. Failed records can be retried from paper detail. Interrupted running records resume on the next launch.
- Evidence comparison: the v2 result must identify at least two paper numbers in each difference, cover every selected paper, and contain source quotes from every paper. A request-specific JSON Schema constrains quotes to literal original passages from their own paper, followed by independent source-attribution validation. Invalid results cause schema validation failure and one repair request with the original sources. Tags are omitted from the comparison input.
- Current LM Studio API: native model loading returns `instance_id`; legacy `model_instance_id` remains accepted. OpenAI-compatible disabled reasoning is `reasoning_effort: "none"`, even when the native model capabilities call this setting `off`. An HTTP 400 may indicate unsupported reasoning settings or insufficient model context, rather than an unavailable server.
- Tag repair: supported English terms may be suggested from the current source excerpt only; managed vocabulary from other papers must never become evidence. The model still selects and returns the tags, and all tags still pass source validation.

- Database: use **Test Database** in Diagnostics to run an integrity check and inspect migration/table counts.
- LM Studio: start its local server, then use **加载并测试**. The app lists downloaded LLMs through the native management API and explicitly loads the selected model. `LMSTUDIO_CONNECTION_FAILED` normally means the server is not listening; `LMSTUDIO_MODEL_NOT_FOUND` can also indicate an unavailable model file or insufficient memory/VRAM.
- Mock AI: select **Mock (development)** in Settings to test views, persistence, parsing, and comparison without LM Studio.
- PDF: inspect extraction character count, detected sections, warnings, and fallback status. `PDF_TEXT_EMPTY` usually indicates a scanned/image-only PDF.
- PDF parser isolation: malformed object trees or unsupported font encodings are contained to one document and create a fallback record marked `PDF_READ_FAILED`; the rest of a folder import continues.
- AI JSON: `AI_INVALID_JSON` and `AI_SCHEMA_VALIDATION_FAILED` record parse/retry status without recording the model reasoning or full response.
- Chinese output/tags: English-only narrative output, tags lacking paper evidence, or fewer than three supported tags are rejected and sent through one constrained repair retry. The managed vocabulary is naming context only and is deliberately excluded from tag evidence checks.

Common codes include `FILE_NOT_FOUND`, `DUPLICATE_PAPER`, `PDF_READ_FAILED`, `PDF_TEXT_EMPTY`, `DATABASE_READ_FAILED`, `DATABASE_WRITE_FAILED`, `LMSTUDIO_CONNECTION_FAILED`, `LMSTUDIO_MODEL_NOT_FOUND`, `LMSTUDIO_INVALID_RESPONSE`, `AI_FINAL_CONTENT_EMPTY`, `AI_OUTPUT_TOKEN_LIMIT`, `MODEL_STRUCTURED_OUTPUT_INCOMPATIBLE`, `AI_TIMEOUT`, `AI_CANCELLED`, `AI_INVALID_JSON`, `AI_SCHEMA_VALIDATION_FAILED`, and `INVALID_SETTINGS`.

## Verification

```powershell
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

For a read-only real-document smoke test, set `PAPER_MASTER_TEST_CORPUS` as documented in `README.md`. The harness creates an in-memory database, exercises recursive import, extraction diagnostics, FTS search and duplicate detection, then leaves the source directory untouched.
