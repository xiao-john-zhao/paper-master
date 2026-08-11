# Changelog

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
