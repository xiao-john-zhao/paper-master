import type { AIProvider } from "../providers/ai/AIProvider";
import type { Paper } from "../domain/paper/Paper";
import type { PaperRepository } from "../repositories/PaperRepository";
import { createOperationId } from "./PaperService";
import { invokeCommand, isTauriRuntime } from "../adapters/tauri/invokeCommand";
import { progressService } from "./ProgressService";
import { normalizeAppError } from "../types/errors";

export class AnalysisService {
  constructor(private provider: AIProvider, private readonly repository: PaperRepository) {}

  setProvider(provider: AIProvider) { this.provider = provider; }

  async analyze(paper: Paper, requestId = createOperationId("analysis"), ensureOnly = false) {
    progressService.begin(requestId,"analysis");
    try {
    const analysis = await this.provider.analyzePaper({ paper_id: paper.id, request_id: requestId, ensure_only: ensureOnly });
    const current = await this.repository.findById(paper.id);
    if (!current) throw new Error("论文已从文献库移除。");
    const updated: Paper = { ...current, analysis, tags: analysis.tags, analysis_status: "analyzed", analysis_error: undefined, updated_at: new Date().toISOString() };
    const saved = await this.repository.update(updated);
    progressService.finish(requestId,"completed","分析完成，已保存到论文库");
    return { paper:saved, requestId };
    } catch (error) { const appError=normalizeAppError(error); progressService.finish(requestId,appError.code === "AI_CANCELLED" ? "cancelled" : "failed",appError.message); throw error; }
  }

  cancel(requestId: string) { return this.provider.cancel(requestId); }
  cancelAutomatic(paperId: string) {
    return isTauriRuntime() ? invokeCommand<void>("cancel_paper_analysis", { paperId }) : Promise.resolve();
  }
}
