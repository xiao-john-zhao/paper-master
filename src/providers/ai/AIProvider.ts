import type { PaperAnalysisResult } from "../../domain/paper/Paper";
import type { AnalysisRequest, CompareRequest, ConnectionResult, PaperComparisonResult } from "../../domain/ai/types";

export interface AIProvider {
  readonly id: "lmstudio" | "mock";
  checkConnection(): Promise<ConnectionResult>;
  getModels(): Promise<string[]>;
  analyzePaper(request: AnalysisRequest): Promise<PaperAnalysisResult>;
  generateTags(paperId: string, requestId: string): Promise<string[]>;
  comparePapers(request: CompareRequest): Promise<PaperComparisonResult>;
  cancel(requestId: string): Promise<void>;
  chatWithPaper?(paperId: string, message: string): Promise<string>;
}

