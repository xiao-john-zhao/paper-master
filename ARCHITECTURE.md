# Architecture

## Dependency direction

```text
React views
  -> application services
    -> domain interfaces
      -> Tauri/SQLite/PDF/LM Studio adapters
```

Views never call SQLite, parse PDFs, or perform LM Studio HTTP requests. Tauri commands are the desktop boundary. Rust infrastructure owns the local database, filesystem parsing, HTTP transport, and durable diagnostics.

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

The project avoids a general-purpose plugin framework in v0.1. Explicit interfaces and registries provide sufficient replaceability.

