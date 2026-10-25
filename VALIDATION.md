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
