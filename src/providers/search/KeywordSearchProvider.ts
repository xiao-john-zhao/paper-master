import { invokeCommand } from "../../adapters/tauri/invokeCommand";
import type { Paper, PaperQuery } from "../../domain/paper/Paper";
import type { SearchProvider } from "./SearchProvider";

export class KeywordSearchProvider implements SearchProvider {
  readonly id = "keyword";

  search(query: PaperQuery) {
    return invokeCommand<Paper[]>("search_papers", { query });
  }
}

