import type { PaperAnalysisResult } from "../../domain/paper/Paper";
import type { AnalysisRequest, CompareRequest, ConnectionResult, PaperComparisonResult } from "../../domain/ai/types";
import type { AIProvider } from "./AIProvider";

const MOCK_ANALYSIS: PaperAnalysisResult = {
  schema_version: "1",
  research_problem: "How can long research documents be represented efficiently while preserving important scientific context?",
  core_method: "A section-aware representation pipeline combines selected abstract, introduction, and conclusion content with bounded supporting passages.",
  key_contributions: [
    "A reproducible section-aware document representation strategy.",
    "A bounded input construction method suitable for local language models.",
    "An evaluation protocol separating extraction and analysis failures.",
  ],
  main_results: "The method improves summary consistency in the reported evaluation while reducing the amount of text processed per paper.",
  limitations: "The evaluation focuses on English, text-based PDFs and does not cover scanned documents.",
  tags: ["document analysis", "local LLM", "scientific NLP", "summarization"],
  ai_summary: "A section-aware, bounded pipeline for reliable local analysis of scientific papers.",
};

function delay(milliseconds: number) {
  return new Promise((resolve) => globalThis.setTimeout(resolve, milliseconds));
}

export class MockAIProvider implements AIProvider {
  readonly id = "mock" as const;
  private cancelled = new Set<string>();

  async checkConnection(): Promise<ConnectionResult> {
    return { state: "connected", message: "Mock provider is ready.", latency_ms: 8, models: ["paper-master-mock-v1"] };
  }

  async getModels() {
    return ["paper-master-mock-v1"];
  }

  async analyzePaper(request: AnalysisRequest) {
    await delay(650);
    if (this.cancelled.delete(request.request_id)) throw new Error("Analysis cancelled");
    return structuredClone(MOCK_ANALYSIS);
  }

  async generateTags() {
    return [...MOCK_ANALYSIS.tags];
  }

  async comparePapers(request: CompareRequest): Promise<PaperComparisonResult> {
    await delay(500);
    if (this.cancelled.delete(request.request_id)) throw new Error("Comparison cancelled");
    return {
      common_points: ["Both papers target reliable processing of scientific documents.", "Both emphasize reproducible local workflows."],
      key_differences: ["Paper A focuses on section-aware input construction; Paper B focuses on evaluation and retrieval."],
      method_comparison: "Paper A uses deterministic section selection, whereas Paper B combines retrieval scores with a learned ranker.",
      contribution_comparison: "The first contribution is primarily a data preparation method; the second is an evaluation and retrieval framework.",
      summary: "The papers are complementary: one improves what enters a local model, and the other improves how supporting evidence is selected.",
    };
  }

  async cancel(requestId: string) {
    this.cancelled.add(requestId);
  }
}
