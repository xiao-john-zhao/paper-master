# Changelog

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
