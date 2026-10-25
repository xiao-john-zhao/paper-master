# Current release: v0.4.1 (2026-10-25)

Next maintenance target: v0.4.2. Version-neutral local recovery files, automatic startup recovery and editable analysis preservation are implemented in v0.4.1. Unified application/network gate, multi-model results/editing, canonical keywords, candidate-team graph and full local-corpus archive are implemented. Follow-up quality work: improve metadata extraction for the 143 flagged PDF records, extend reviewed vocabulary, and revalidate the complete local-model pipeline after runtime recovery. Historical accepted scopes follow below.


# Development plan

- Current Version: `v0.3.1`
- Current Phase: local takeover and v0.3.2 feature verification
- Current Stable Reference: `v0.3.1`
- Next Target Version: `v0.3.2`

The project follows a stable-core / extensible-adapter design. A phase is complete only after its build and relevant tests pass and the prior behavior remains intact.

| Phase | Scope | Status |
| --- | --- | --- |
| 1 | Environment, Tauri/React bootstrap, Git, config, errors, logging | Complete |
| 2 | Domain interfaces, three-pane UI, Settings and Diagnostics | Complete |
| 3 | SQLite migrations, repository, CRUD and integrity checks | Complete |
| 4 | PDF/folder import, extraction, sections, open PDF | Complete |
| 5 | LM Studio + Mock providers, analysis, validation, tracing | Complete |
| 6 | Keyword and tag search | Complete |
| 7 | Structured comparison for 2–8 papers | Complete |
| 8 | Unit/integration tests, diagnostics and failure handling | Complete |
| 9 | UX polish, Windows build and demo validation | Complete |

## Accepted v0.3.2 scope (2026-10-22)

- Automatic local LM Studio analysis after first PDF import; durable sequential queue, cancellation, retry and interrupted-job recovery.
- Model-generated semantic comparison of 2–8 papers, using summaries plus bounded original evidence, six difference dimensions and validated per-paper source quotes.
- Current LM Studio API compatibility, explicit context/reasoning controls, reproducible local build and startup scripts.
- Preserve source PDFs and existing databases. Validate deterministic tests, real read-only PDF import and real local-model inference before handoff. Version sources remain at v0.3.1 until a release/version decision.

## Definition of done

2026-10-23 用户追加范围：PDF 打开修复；可选 Codex IEEE 检索与论文阅读总结；公开 PDF 下载、学校 WebVPN 试验下载；摘要清理与旧库迁移；作者身份/通信证据/本地关联论文；主程序之外的私有凭据 TXT 与迁移排除。学校订阅自动下载未完成端到端验证的部分须保留在独立“问题清单”文件夹，不能据仅打开入口宣称成功。

- Frontend TypeScript build passes.
- Rust checks and tests pass.
- No paper content or complete prompt is written to logs.
- Errors have a stable code and friendly UI message.
- Cross-module operations have a request/operation ID.
- Documentation reflects the implemented behavior.
