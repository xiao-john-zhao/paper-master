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

The Diagnostics JSONL transport event records `request_id`, `provider`, `method`, `endpoint`, `model`, `http_status`, and `duration_ms`. It never records the full paper text or complete prompt.

## Subsystem checks

- Database: use **Test Database** in Diagnostics to run an integrity check and inspect migration/table counts.
- LM Studio: start its local server, then use **加载并测试**. The app lists downloaded LLMs through the native management API and explicitly loads the selected model. `LMSTUDIO_CONNECTION_FAILED` normally means the server is not listening; `LMSTUDIO_MODEL_NOT_FOUND` can also indicate an unavailable model file or insufficient memory/VRAM.
- Mock AI: select **Mock (development)** in Settings to test views, persistence, parsing, and comparison without LM Studio.
- PDF: inspect extraction character count, detected sections, warnings, and fallback status. `PDF_TEXT_EMPTY` usually indicates a scanned/image-only PDF.
- PDF parser isolation: malformed object trees or unsupported font encodings are contained to one document and create a fallback record marked `PDF_READ_FAILED`; the rest of a folder import continues.
- AI JSON: `AI_INVALID_JSON` records parse/retry status without recording the model reasoning or full response.
- Chinese output/tags: English-only narrative output, tags lacking paper evidence, or fewer than three supported tags are rejected and sent through one constrained repair retry. The managed vocabulary is naming context only and is deliberately excluded from tag evidence checks.

Common codes include `FILE_NOT_FOUND`, `DUPLICATE_PAPER`, `PDF_READ_FAILED`, `PDF_TEXT_EMPTY`, `DATABASE_READ_FAILED`, `DATABASE_WRITE_FAILED`, `LMSTUDIO_CONNECTION_FAILED`, `LMSTUDIO_MODEL_NOT_FOUND`, `LMSTUDIO_INVALID_RESPONSE`, `AI_TIMEOUT`, `AI_CANCELLED`, `AI_INVALID_JSON`, and `INVALID_SETTINGS`.

## Verification

```powershell
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

For a read-only real-document smoke test, set `PAPER_MASTER_TEST_CORPUS` as documented in `README.md`. The harness creates an in-memory database, exercises recursive import, extraction diagnostics, FTS search and duplicate detection, then leaves the source directory untouched.
