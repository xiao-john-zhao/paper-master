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
  tags: z.array(z.string().trim().min(1)).min(3).max(8),
  ai_summary: z.string().trim().optional(),
});

export const paperComparisonSchema = z.object({
  common_points: z.array(z.string().trim().min(1)),
  key_differences: z.array(z.string().trim().min(1)),
  method_comparison: z.string().trim().min(1),
  contribution_comparison: z.string().trim().min(1),
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
}

export interface CompareRequest {
  paper_ids: string[];
  request_id: string;
}
