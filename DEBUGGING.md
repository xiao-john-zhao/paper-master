# Debugging

## Development mode

Run `pnpm tauri dev`. Development builds include frontend source maps and the WebView inspector can be opened with `Ctrl+Shift+I` or the context-menu **Inspect** action. Set `RUST_BACKTRACE=1` before starting the app to include Rust backtraces in the terminal during development.

## Logs

The Diagnostics page reports the exact log path. Logs are JSON Lines and include timestamp, level, module, operation, error code, request ID, and duration where relevant. They intentionally exclude full paper text, file content, complete AI prompts, and model reasoning.

Use a request ID such as `analysis-YYYYMMDD-xxxx` to trace UI, provider, parser, and database events for one operation.

## Subsystem checks

- Database: use **Test Database** in Diagnostics to run an integrity check and inspect migration/table counts.
- LM Studio: start its local server, then use **Test Connection**. `LMSTUDIO_CONNECTION_FAILED` normally means the server is not listening; `LMSTUDIO_MODEL_NOT_FOUND` means no model is loaded/selected.
- Mock AI: select **Mock (development)** in Settings to test views, persistence, parsing, and comparison without LM Studio.
- PDF: inspect extraction character count, detected sections, warnings, and fallback status. `PDF_TEXT_EMPTY` usually indicates a scanned/image-only PDF.
- AI JSON: `AI_INVALID_JSON` records parse/retry status without recording the model reasoning or full response.

Common codes include `FILE_NOT_FOUND`, `DUPLICATE_PAPER`, `PDF_READ_FAILED`, `PDF_TEXT_EMPTY`, `DATABASE_READ_FAILED`, `DATABASE_WRITE_FAILED`, `LMSTUDIO_CONNECTION_FAILED`, `LMSTUDIO_MODEL_NOT_FOUND`, `AI_TIMEOUT`, `AI_CANCELLED`, `AI_INVALID_JSON`, and `INVALID_SETTINGS`.

