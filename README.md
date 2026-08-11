# paper-master

`paper-master` is a Windows desktop research-paper library built with Tauri 2, React, TypeScript, Rust, and SQLite. It is local-first: imported PDFs remain in place, metadata and analysis stay in a local database, and AI requests are sent only to the user's LM Studio server on `localhost`.

## v0.2 Demo

Version `0.3.0` is a runnable Windows demo with:

- recursive PDF/folder import, duplicate detection, resilient text extraction, metadata fallbacks and per-file diagnostics;
- a durable SQLite library with migrations, CRUD, FTS5 keyword search, filters and tags;
- LM Studio downloaded-model discovery, explicit model loading, structured Chinese analysis, cancellation and JSON validation/repair;
- evidence-constrained managed tags and structured comparison of 2–8 papers;
- a deterministic Mock provider for development and failure-path testing;
- centralized error codes, request-ID JSONL tracing and a development Diagnostics page.

Imported PDFs are referenced in place—the application never moves or deletes the original file.

## Development prerequisites

- Windows 10 or 11 with WebView2
- Node.js 20+ and pnpm
- Rust stable MSVC toolchain
- Visual Studio C++ Build Tools required by Tauri
- LM Studio (optional for UI development; `MockAIProvider` is available)

## Commands

```powershell
pnpm install
pnpm test
pnpm build
pnpm tauri dev
pnpm tauri build
```

The Windows installer is written to `src-tauri/target/release/bundle/nsis/`.
The final standalone executable is also copied to `application/paper-master-v0.3.0.exe` for convenient access.

## LM Studio setup

1. Load an instruction-capable model in LM Studio and start its local server.
2. Open **Settings** in paper-master.
3. Keep the base URL on loopback (the default is `http://127.0.0.1:1234/v1`), choose **加载并测试**, select a discovered model, and save. paper-master uses LM Studio's native model-management API to load the selected model before inference.
4. Import a PDF and choose **AI Analyze**. Stored structured analyses can be searched and compared later.

The LM Studio adapter rejects non-loopback hosts. Paper content is never sent to a cloud endpoint by this demo. Summaries, tags and multi-paper comparisons are generated in Simplified Chinese; technical acronyms and proper names may remain in their source form.

## Local data and privacy

On Windows, SQLite data and JSONL diagnostics live under the Tauri application-data directory for `app.papermaster.desktop`. The Diagnostics page shows the exact paths and can open the log folder. Logs contain operation metadata—not PDF text, complete prompts, raw model responses, or reasoning.

## Real-corpus smoke test

Set `PAPER_MASTER_TEST_CORPUS` to a directory of PDFs, then run Rust tests with output enabled. The test uses an in-memory database and does not modify the source PDFs.

```powershell
$env:PAPER_MASTER_TEST_CORPUS = "E:\path\to\papers"
cargo test --manifest-path src-tauri/Cargo.toml imports_configured_real_corpus -- --nocapture
```

No paper PDFs, model files, databases, or local logs are committed to the repository.
