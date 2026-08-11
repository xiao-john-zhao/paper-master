# v0.1 validation

Validated on Windows with the production Tauri build.

## Automated checks

- Frontend: 8 Vitest tests passed across schema, repository/search, analysis persistence, and comparison preconditions.
- Rust: 21 tests passed across migrations, SQLite/FTS, PDF cleaning and section detection, local-only settings, AI input/parser contracts, cancellation, import fallbacks, and duplicate detection.
- TypeScript/Vite production build completed successfully.
- Tauri release compilation and NSIS x64 installer packaging completed successfully.

## Real PDF corpus

The recursive import smoke test used 12 local PDFs through the production import/parser/repository code path and an in-memory validation database. Source files were read-only.

- 12/12 paper records imported; 0 import failures.
- 2 structurally incompatible PDFs were isolated and saved as fallback records instead of aborting the batch.
- Re-import detected 12/12 duplicates.
- FTS query `communication` matched 10 papers.

This corpus exposed two third-party parser panics (an object-index edge case and an unsupported `UniGB-UCS2-H` font encoding). The document parser boundary now catches these failures, records `PDF_READ_FAILED` diagnostics, and allows the remaining import to continue.

## Release smoke test

The release executable started successfully, created/opened its Windows application-data database, migrated the schema to version 2, and appended startup/migration events to the JSONL log. The smoke-test process was then closed normally by the validation harness.

LM Studio was not running during final packaging; local model inference therefore remains an environment-dependent manual acceptance step. The deterministic Mock provider and structured-output parser paths are covered by automated tests.
