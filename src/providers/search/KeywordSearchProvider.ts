import { invokeCommand } from "../../adapters/tauri/invokeCommand";
import type { Paper, PaperQuery } from "../../domain/paper/Paper";
import type { SearchProvider } from "./SearchProvider";
import type { PaperRepository } from "../../repositories/PaperRepository";

export class KeywordSearchProvider implements SearchProvider {
  readonly id = "keyword";

  constructor(private readonly repository?: PaperRepository) {}

  search(query: PaperQuery) {
    return this.repository?.search(query) ?? invokeCommand<Paper[]>("search_papers", { query });
  }
}
