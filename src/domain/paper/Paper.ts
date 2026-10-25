export type AnalysisStatus = "not_analyzed" | "queued" | "analyzing" | "analyzed" | "failed";

export interface PdfDiagnostics {
  page_count: number;
  extracted_character_count: number;
  detected_sections: string[];
  parser_status: "success" | "partial" | "failed";
  warnings: string[];
  fallback_used: boolean;
}

export interface PaperAnalysisResult {
  schema_version: "1" | "2";
  research_problem: string;
  research_scenario?: string;
  core_method: string;
  key_innovations?: string[];
  key_contributions: string[];
  main_results: string;
  limitations: string;
  tags: string[];
  ai_summary?: string;
}

export interface Paper {
  id: string;
  file_path: string;
  file_name: string;
  title: string;
  publication_venue?: string;
  authors: string[];
  year?: number;
  abstract?: string;
  analysis?: PaperAnalysisResult;
  analysis_provider?: string;
  tags: string[];
  analysis_status: AnalysisStatus;
  analysis_error?: string;
  pdf_diagnostics?: PdfDiagnostics;
  created_at: string;
  updated_at: string;
}

export type EditableMetadata = Pick<Paper,"title"|"authors"|"abstract"|"publication_venue"|"year">;

export type LibraryFilter = "all" | "recent" | "analyzed";

export interface PaperQuery {
  text?: string;
  tag?: string;
  tags?: string[];
  filter?: LibraryFilter;
}

export interface ImportSummary {
  operation_id: string;
  imported: Paper[];
  duplicates: string[];
  failed: Array<{ path: string; code: string; message: string }>;
}
