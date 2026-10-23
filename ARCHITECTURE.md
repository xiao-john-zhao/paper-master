# v0.4.0 additions

Migration 8 cleans spurious IEEE membership author fragments in automatically extracted metadata; manually edited metadata is preserved.

- analysis_versions / active_analyses: ordered migration 7, immutable generated baseline on first edit, optimistic revisions, atomic selection/search projection and cascade deletion. Paper.analysis_provider distinguishes extractive records from model outputs.
- AnalysisVersionsService and MetadataEditor route UI mutations through application services and scoped Rust commands. Metadata patches read current rows; generated comparison originals and editable narratives persist locally in settings records.
- A persisted privacy_mode is the one network gate. Offline Codex selection resolves to loopback LM Studio; settings revocation cancels registered work and closes the remote WebVPN window.
- keywords.rs owns conservative canonical aliases; normalization follows model evidence validation. archive.rs builds explicitly non-model excerpts, repairs only unedited missing metadata and supports idempotent batch archiving. Optional local pdftotext has bounded runtime/output and Rust fallback.
- authors::team_links precomputes author evidence once per document; graph services supply evidence to the pure layout module. Potential-team mode prioritizes connected records and renders distinct dashed edges.


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

- `ProgressService`: session-scoped task snapshots subscribed through `useSyncExternalStore`; native `task-progress` events contain only IDs, stages and counts. Background imports run in a blocking worker and emit per-file completion even for duplicates/fallbacks. Public downloads emit throttled received-byte counts when the response has a total; model operations report stages rather than inferred generation percentages. Terminal states reject late running events.
- `RelationshipGraphService`: reads the complete paper library, coordinates pure domain graph counting/layout and image export. Keywords count once per paper, co-occurrence edges retain supporting paper IDs, and paper edges carry shared tags/names. Domain normalization is exact after case/space normalization, not semantic disambiguation. Graphs are limited to 80 nodes/240 strongest edges with visible coverage notes. SVG/PNG exports include complete label legends; the Rust save boundary permits only PNG/SVG extensions and validates format/size before a native save dialog. Blob image CSP support is restricted to local rasterization.
- `AIProvider`: connection, model discovery, paper analysis, tags and comparison.
- `SearchProvider`: search papers independently of the current keyword implementation.
- `PaperRepository`: create, update, delete, read, list, search and duplicate checks.
- `DocumentParser`: parse an input document into cleaned text, sections, metadata and diagnostics.
- `SettingsStorage`: read and validate local provider/runtime settings.

Interfaces return domain types or a centralized error type. Optional fields may be added compatibly; incompatible analysis output uses a new schema/prompt version.

## Extension points

Codex is an opt-in online adapter registered as `codex`. It runs an installed signed-in executable without a shell, feeds bounded inputs through stdin, uses ephemeral sessions and strict output schemas, and discards process logs. Analysis passes the existing Chinese/tag/evidence validators. Research first interprets a natural-language request into a versioned intent (topic, target authors, reference, relationship and years), then performs web retrieval with that plan. It receives the request and optional bounded title/author metadata, never school credentials or reference full text. Ambiguous references return a clarification without web retrieval. Backend filters enforce names/relationships/year conditions, recompute shared author names and reject topic-only substitutes for team requests. `local_only` remains the default; `codex_online` must be explicitly saved before search or Codex analysis.

School WebVPN uses an external Tauri window without IPC capabilities, with a private browser profile outside the project. Its download callback checks a new file before importing through the existing import service. Public downloads use HTTPS host/redirect/size/PDF-header validation. System-browser access is the fallback for existing school sessions. The credential TXT is opened locally and is never returned through IPC. Migration 6 repairs stored abstracts without changing analyses or source PDF files.

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
- New text-bearing imports are saved as `queued`. A backend worker drains this durable queue serially; interrupted `analyzing` jobs return to `queued` at startup. Manual analysis and comparison share a cancellable inference semaphore. Preparation errors become visible failed states without blocking later papers.
- Comparison accepts 2–8 unique papers. The application service ensures missing analyses are completed first. The backend fairly shares a single total input budget across saved analyses (excluding tags) and bounded original-text excerpts for every paper. The local model performs a fresh semantic comparison; v2 results contain six comparison dimensions and per-paper verbatim evidence. Validators check paper-number coverage, quote attribution and exact source support. Full unbounded extracted text is not sent.
- SQLite migration 2 maintains an FTS5 index with triggers. Search input is sanitized and bounded, with field matching as a safe fallback.
- UI, provider transport, JSON validation and repository-save events share one request ID. Log events omit document content and complete prompts.

The project avoids a general-purpose plugin framework in v0.1. Explicit interfaces and registries provide sufficient replaceability.
