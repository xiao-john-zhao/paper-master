import { z } from "zod";

export const paperAnalysisSchema = z.object({
  schema_version: z.literal("2").default("2"),
  research_problem: z.string().trim().min(1),
  research_scenario: z.string().trim().min(1),
  core_method: z.string().trim().min(1),
  key_innovations: z.array(z.string().trim().min(1)).min(1),
  key_contributions: z.array(z.string().trim().min(1)).min(1),
  main_results: z.string().trim().min(1),
  limitations: z.string().trim().min(1),
  tags: z.array(z.string().trim().min(1)).min(1).max(8),
  ai_summary: z.string().trim().min(1),
});

export const paperComparisonSchema = z.object({
  schema_version: z.literal("2"),
  common_points: z.array(z.string().trim().min(1)),
  key_differences: z.array(z.string().trim().min(1)),
  problem_comparison: z.string().trim().min(1),
  scenario_comparison: z.string().trim().min(1),
  method_comparison: z.string().trim().min(1),
  contribution_comparison: z.string().trim().min(1),
  results_comparison: z.string().trim().min(1),
  limitations_comparison: z.string().trim().min(1),
  evidence: z.array(z.object({ paper_number: z.number().int().min(1).max(8), quote: z.string().min(20).max(180) })),
  summary: z.string().trim().min(1),
});

export type PaperAnalysisResultV2 = z.infer<typeof paperAnalysisSchema>;
export type PaperComparisonResult = z.infer<typeof paperComparisonSchema>;

export type ConnectionState = "connected" | "disconnected" | "model_not_loaded" | "connection_failed";

export interface ConnectionResult {
  state: ConnectionState;
  message: string;
  latency_ms?: number;
  models: string[];
  loaded_models: string[];
}

export interface AnalysisRequest {
  paper_id: string;
  request_id: string;
  ensure_only?: boolean;
}

export interface CompareRequest {
  paper_ids: string[];
  request_id: string;
}
