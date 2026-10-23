# v0.4.0 — 2026-10-24

- 统一应用及启动入口；保存联网开关后统一拦截外部入口、取消在途任务并关闭学校窗口。
- 兼容迁移 7：按服务/模型/时间保留多份整理结果；切换、逐字段编辑、修订冲突保护及恢复原始生成文字。
- 编辑题名、作者、摘要、出版物、年份及对比叙述；避免旧快照覆盖新结果。
- 受管关键词中英文别名归一、去重和检索；来源明确的本地原文摘录。
- 潜在同团队证据、紫色虚线、独立开关与团队视图；导出同步显示图例。
- Poppler 兼容提取、期刊封面/中文摘要修复；同一个 EXE 提供归档与单篇分析命令。
- 迁移 8 清理 Life Fellow 等 IEEE 会员称谓造成的伪作者，避免错误团队连线，跳过人工编辑记录。
- 全量 439 PDF 实际归档与哈希校验，143 项记录待人工核对；完整结果见 docs/2026-10-24-v0.4.0-unified-archive.md。

以下为旧版本及预发布工作树历史。

# Changelog

## Unreleased — local takeover / v0.3.2 worktree

### Added

- Progress bars and a global task center for imports, queued/manual analysis, comparisons, research, downloads, settings, graph building and image export; actual file/byte/stage counts, elapsed time and terminal states.
- Interactive keyword co-occurrence and paper relationship graphs using local tags/shared author names, bounded deterministic layout, filters, pan/zoom, evidence inspection, paper navigation, SVG and PNG export with full label legends.
- Larger and more consistent typography, wrapping paper titles and responsive graph panels.
- Optional Codex CLI provider with stdin prompts, ephemeral sessions, cancellation/timeouts and validated Chinese paper analysis; explicit online-mode setting.
- Natural-language research requests with optional reference papers: interpret topic/author/team/year constraints before web retrieval, display the interpretation and per-result relationship evidence, ask for missing references and enforce filters without substituting topic-only matches.
- IEEE research page with source-backed candidate screening, public HTTPS PDF download/import, isolated experimental WebVPN download window and system-browser fallback.
- Separate author/first-author/corresponding-author evidence and library-wide shared-author or potential-team candidates. Private credential TXT opener and transfer exclusions.

- Automatically queue new text-bearing imports for local AI identification and Chinese summaries; serialize inference, resume interrupted jobs, expose queue/failure states, and support cancellation/retry.
- Prepare missing analyses before multi-paper comparison. Generate semantic differences across research problems, scenarios, mechanisms, contributions, results and limitations using saved summaries and bounded raw excerpts, with validated verbatim evidence from every selected paper.
- Configurable import automation, model context length and reasoning effort; local build/start scripts.

### Fixed

- Finish interrupted WebVPN download progress, keep late task events from reviving completed/cancelled tasks, and initialize browser previews with the Mock provider.
- Generate a Common Controls v6 manifest for MSVC test harnesses so linking native save dialogs does not cause `STATUS_ENTRYPOINT_NOT_FOUND` at test startup.
- Open library PDFs through a validated backend record rather than the unscoped frontend opener; support relinking moved PDFs.
- Stop abstracts at keywords and author/affiliation footnotes, accept long inline abstract headings, and repair existing abstracts with ordered migration 6.

- Accept both current LM Studio `instance_id` and legacy `model_instance_id` load responses.
- Send `none` for disabled reasoning through the OpenAI-compatible endpoint and include original evidence when requesting comparison repair.
- Constrain generated citations to per-paper literal source passages in the response Schema, preventing the model from paraphrasing quoted evidence.
- Keep cancelled comparisons from overwriting a newer result, and use backend-backed Mock behavior consistently in the desktop app.

## v0.3.1

### Fixed

- Raised the default LM Studio request timeout from 120 seconds to 600 seconds for long-running local paper analysis.
- Added backend and frontend regression assertions for the 600-second default.

## v0.3.0

### Added

- Added persisted publication venue metadata with best-effort extraction of journal, conference, workshop, proceedings, or preprint names from imported PDFs.
- Added a backward-compatible database migration that backfills venue metadata from existing locally stored PDF text when possible and preserves an explicit fallback when no venue can be identified.

### Changed

- Reordered the left paper preview to show title, publication venue, and authors as its first three rows.
- Removed the PDF file name and publication year from the compact preview.
- Displayed keyword tags in the preview only after AI analysis has completed.
- Improved PDF metadata detection to skip running journal/page headers, join multiline paper titles, and remove IEEE membership labels from author names.
- Backfilled legacy title fields from existing extracted text so previously imported papers no longer keep a filename or journal header as their display title when the real title can be detected.

## v0.2.1

### Fixed

- Treat HTTP 200 responses with empty `choices[0].message.content` as `AI_FINAL_CONTENT_EMPTY` without using or logging reasoning text.
- Distinguish output-token exhaustion, invalid JSON syntax, schema validation failures, and structured-output incompatibility.
- Request LM Studio JSON Schema structured output generated from the Rust response types and apply a configurable output-token limit.

### Added

- Record privacy-safe response diagnostics and explicit analysis-pipeline stages without prompts, paper text, or reasoning content.
- Cover final-content extraction, schema strictness, and optional live LM Studio smoke tests.

## v0.2.0

### Fixed

- Unified LM Studio OpenAI-compatible Base URL handling so model discovery uses `/v1/models` and chat completion uses `/v1/chat/completions`.
- Added strict validation for HTTP 200 Chat Completions responses and the `LMSTUDIO_INVALID_RESPONSE` error.
- Removed the incompatible `response_format=json_object` request option after verification against the installed LM Studio server.
- Restored scrolling for overflowing library, paper detail, Settings, Diagnostics, and comparison content.
- Kept the Settings navigation entry visible when the application window has limited vertical space.

### Added

- Added endpoint normalization and Chat Completions response-envelope unit tests.
- Added LM Studio transport diagnostics for method, endpoint, model, HTTP status, request ID, and duration without logging paper or prompt content.
