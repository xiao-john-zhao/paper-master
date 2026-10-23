import type { ImportSummary, Paper, PaperQuery } from "../domain/paper/Paper";
import type { PaperRepository } from "../repositories/PaperRepository";
import type { SearchProvider } from "../providers/search/SearchProvider";
import { invokeCommand } from "../adapters/tauri/invokeCommand";
import type { AuthorContext } from "../domain/paper/AuthorContext";
import { progressService } from "./ProgressService";
import { normalizeAppError } from "../types/errors";

export class PaperService {
  constructor(private readonly repository: PaperRepository, private readonly searchProvider?: SearchProvider) {}

  list(query: PaperQuery = {}) {
    return query.text && this.searchProvider ? this.searchProvider.search(query) : this.repository.list(query);
  }

  get(id: string) { return this.repository.findById(id); }
  save(paper: Paper) { return this.repository.update(paper); }
  remove(id: string) { return this.repository.delete(id); }
  openPdf(id: string) { return invokeCommand<void>("open_paper_pdf", { id }); }
  relinkPdf(id: string, path: string) { return invokeCommand<Paper>("relink_paper_pdf", { id, path }); }
  authorContext(id: string) { return invokeCommand<AuthorContext>("get_author_context", { id }); }

  async importPaths(paths: string[], operationId = createOperationId("import")): Promise<ImportSummary> {
    progressService.begin(operationId,"import");
    try { const result = await this.repository.create(paths, operationId); progressService.finish(operationId,result.failed.length && !result.imported.length ? "failed" : "completed",`导入 ${result.imported.length} 篇，跳过 ${result.duplicates.length} 篇，失败 ${result.failed.length} 项`); return result; }
    catch (error) { progressService.finish(operationId,"failed",normalizeAppError(error).message); throw error; }
  }
}

export function createOperationId(prefix: "analysis" | "compare" | "import") {
  const stamp = new Date().toISOString().replace(/[-:TZ.]/g, "").slice(0, 14);
  return `${prefix}-${stamp}-${crypto.randomUUID().slice(0, 8)}`;
}
