# paper-master

`paper-master` is a Windows desktop research-paper library built with Tauri 2, React, TypeScript, Rust, and SQLite. It is local-first: imported PDFs remain in place, metadata and analysis stay in a local database, and AI requests are sent only to the user's LM Studio server on `localhost`.

## Status

Version `0.1.0` is under active development. See [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md) for phase status.

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

No paper PDFs, model files, databases, or local logs are committed to the repository.

