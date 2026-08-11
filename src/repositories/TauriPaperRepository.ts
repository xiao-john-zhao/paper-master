import { invokeCommand } from "../adapters/tauri/invokeCommand";
import type { ImportSummary, Paper, PaperQuery } from "../domain/paper/Paper";
import type { PaperRepository } from "./PaperRepository";

export class TauriPaperRepository implements PaperRepository {
  create(paths: string[], operationId: string) {
    return invokeCommand<ImportSummary>("import_papers", { paths, operationId });
  }
  update(paper: Paper) { return invokeCommand<Paper>("update_paper", { paper }); }
  delete(id: string) { return invokeCommand<void>("delete_paper", { id }); }
  findById(id: string) { return invokeCommand<Paper | null>("get_paper", { id }); }
  list(query: PaperQuery = {}) { return invokeCommand<Paper[]>("list_papers", { query }); }
  search(query: PaperQuery) { return invokeCommand<Paper[]>("search_papers", { query }); }
  existsByPath(path: string) { return invokeCommand<boolean>("paper_exists_by_path", { path }); }
}

