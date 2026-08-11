# Architecture

## Dependency direction

```text
React views
  -> application services
    -> domain interfaces
      -> Tauri/SQLite/PDF/LM Studio adapters
```

Views never call SQLite, parse PDFs, or perform LM Studio HTTP requests. Tauri commands are the desktop boundary. Rust infrastructure owns the local database, filesystem parsing, HTTP transport, and durable diagnostics.

```text
src/
  app + pages + features       React presentation
  services                     use-case coordination
  providers + repositories     TypeScript ports/adapters
  domain + settings            versioned contracts
src-tauri/src/
  commands                     IPC boundary
  services                     import workflows
  ai + pdf                     provider/parser implementations
  database                     SQLite repositories + ordered migrations
  diagnostics + error          JSONL tracing + stable errors
```

## Stable interfaces

- `AIProvider`: connection, model discovery, paper analysis, tags and comparison.
- `SearchProvider`: search papers independently of the current keyword implementation.
- `PaperRepository`: create, update, delete, read, list, search and duplicate checks.
- `DocumentParser`: parse an input document into cleaned text, sections, metadata and diagnostics.
- `SettingsStorage`: read and validate local provider/runtime settings.

Interfaces return domain types or a centralized error type. Optional fields may be added compatibly; incompatible analysis output uses a new schema/prompt version.

## Extension points

- New AI service: implement another provider and register it without changing LM Studio.
- Vector or hybrid search: implement another `SearchProvider`; retain keyword search as the fallback.
- Word/Markdown/HTML: implement another `DocumentParser` and register it by file type.
- RAG: add chunking, embedding, index, and retrieval services behind new interfaces. Do not place retrieval in the paper detail view or modify keyword search.
- Database changes: add ordered migrations; never silently recreate the user's database.

## Reliability boundaries

- PDF extraction is isolated per document. A parser error or third-party panic becomes a fallback paper record with diagnostics and cannot abort a folder import.
- AI input is assembled from preferred sections under a configured character budget. Responses must satisfy a versioned schema before persistence.
- LM Studio URLs must resolve to loopback. The provider uses the native `/api/v1/models` catalog and `/api/v1/models/load` endpoint when available, with legacy/OpenAI-compatible discovery fallbacks. The HTTP client ignores system proxies, has bounded timeouts, and supports cancellation.
- Generated narrative fields and comparisons must pass a Simplified-Chinese check. Tags are validated against the current paper content before persistence.
- A bounded managed-tag vocabulary is built from other papers by usage count. It is supplied only as naming context: exact equivalent tags reuse the canonical spelling, while the vocabulary is excluded from evidence validation so unrelated library topics cannot leak into the current paper.
- Comparison accepts 2–8 unique papers and sends only their saved structured analyses, not full extracted text.
- SQLite migration 2 maintains an FTS5 index with triggers. Search input is sanitized and bounded, with field matching as a safe fallback.
- UI, provider transport, JSON validation and repository-save events share one request ID. Log events omit document content and complete prompts.

The project avoids a general-purpose plugin framework in v0.1. Explicit interfaces and registries provide sufficient replaceability.
