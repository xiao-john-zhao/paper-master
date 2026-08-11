import type { ImportSummary, Paper, PaperQuery } from "../domain/paper/Paper";
import type { PaperRepository } from "../repositories/PaperRepository";
import type { SearchProvider } from "../providers/search/SearchProvider";

export class PaperService {
  constructor(private readonly repository: PaperRepository, private readonly searchProvider?: SearchProvider) {}

  list(query: PaperQuery = {}) {
    return query.text && this.searchProvider ? this.searchProvider.search(query) : this.repository.list(query);
  }

  get(id: string) { return this.repository.findById(id); }
  save(paper: Paper) { return this.repository.update(paper); }
  remove(id: string) { return this.repository.delete(id); }

  importPaths(paths: string[], operationId = createOperationId("import")): Promise<ImportSummary> {
    return this.repository.create(paths, operationId);
  }
}

export function createOperationId(prefix: "analysis" | "compare" | "import") {
  const stamp = new Date().toISOString().replace(/[-:TZ.]/g, "").slice(0, 14);
  return `${prefix}-${stamp}-${crypto.randomUUID().slice(0, 8)}`;
}
