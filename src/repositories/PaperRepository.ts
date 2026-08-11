import type { ImportSummary, Paper, PaperQuery } from "../domain/paper/Paper";

export interface PaperRepository {
  create(paths: string[], operationId: string): Promise<ImportSummary>;
  update(paper: Paper): Promise<Paper>;
  delete(id: string): Promise<void>;
  findById(id: string): Promise<Paper | null>;
  list(query?: PaperQuery): Promise<Paper[]>;
  search(query: PaperQuery): Promise<Paper[]>;
  existsByPath(path: string): Promise<boolean>;
}

