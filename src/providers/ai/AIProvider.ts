import type { PaperAnalysisResult } from "../../domain/paper/Paper";
import type { AnalysisRequest, CompareRequest, ConnectionResult, PaperComparisonResult } from "../../domain/ai/types";
import type { AppSettings } from "../../settings/types";

export interface AIProvider {
  readonly id: "lmstudio" | "mock" | "codex";
  checkConnection(settings?: AppSettings): Promise<ConnectionResult>;
  getModels(settings?: AppSettings): Promise<string[]>;
  loadModel(settings?: AppSettings): Promise<ConnectionResult>;
  analyzePaper(request: AnalysisRequest): Promise<PaperAnalysisResult>;
  generateTags(paperId: string, requestId: string): Promise<string[]>;
  comparePapers(request: CompareRequest): Promise<PaperComparisonResult>;
  cancel(requestId: string): Promise<void>;
  chatWithPaper?(paperId: string, message: string): Promise<string>;
}
