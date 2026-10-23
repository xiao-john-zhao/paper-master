export interface AuthorContext {
  authors: string[];
  first_author?: string;
  corresponding_authors: string[];
  corresponding_evidence: string[];
  related_papers: Array<{ id: string; title: string; shared_authors: string[]; relationship: string; evidence: string }>;
}
