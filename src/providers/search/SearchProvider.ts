import type { Paper, PaperQuery } from "../../domain/paper/Paper";

export interface SearchProvider {
  readonly id: string;
  search(query: PaperQuery): Promise<Paper[]>;
}

