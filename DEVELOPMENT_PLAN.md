# Development plan

- Current Version: `v0.3.0`
- Current Phase: maintenance and bug fixing
- Current Stable Commit: `c7745a7`
- Next Target Version: `v0.3.1`

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

## Definition of done

- Frontend TypeScript build passes.
- Rust checks and tests pass.
- No paper content or complete prompt is written to logs.
- Errors have a stable code and friendly UI message.
- Cross-module operations have a request/operation ID.
- Documentation reflects the implemented behavior.
