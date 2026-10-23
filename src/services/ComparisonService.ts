import type { Paper } from "../domain/paper/Paper";
import type { PaperComparisonResult } from "../domain/ai/types";
import type { AIProvider } from "../providers/ai/AIProvider";
import { AppError } from "../types/errors";
import { createOperationId } from "./PaperService";
import type { AnalysisService } from "./AnalysisService";
import type { PaperRepository } from "../repositories/PaperRepository";
import { progressService } from "./ProgressService";
import { normalizeAppError } from "../types/errors";

export class ComparisonService {
  private preparing = new Map<string, string>();
  private cancelled = new Set<string>();
  constructor(private provider: AIProvider, private readonly analysisService?: AnalysisService, private readonly repository?: PaperRepository) {}

  setProvider(provider: AIProvider) { this.provider = provider; }

  async compare(papers: Paper[], requestId = createOperationId("compare")): Promise<{ result: PaperComparisonResult; requestId: string }> {
    if (papers.length < 2 || papers.length > 8 || new Set(papers.map((paper) => paper.id)).size !== papers.length) {
      throw new AppError({ code: "PAPER_NOT_ANALYZED", message: "请选择 2–8 篇互不相同的论文进行对比。" });
    }
    this.cancelled.delete(requestId);
    progressService.begin(requestId,"compare");
    try {
      for (const [index,paper] of papers.entries()) {
        progressService.update({id:requestId,kind:"compare",stage:"检查并补齐各篇论文的内容分析",completed:index,total:papers.length,unit:"files"});
        if (this.cancelled.has(requestId)) throw new AppError({ code: "AI_CANCELLED", message: "论文对比已取消。" });
        const current = this.repository ? await this.repository.findById(paper.id) : paper;
        if (!current) throw new AppError({ code: "FILE_NOT_FOUND", message: "所选论文已移除。" });
        if (!current.analysis || current.analysis_status !== "analyzed") {
          if (!this.analysisService) throw new AppError({ code: "PAPER_NOT_ANALYZED", message: `请先分析《${current.title}》，再执行多篇论文对比。` });
          const preparationId = `${requestId}-prepare-${paper.id}`;
          this.preparing.set(requestId, preparationId);
          await this.analysisService.analyze(current, preparationId, true);
          this.preparing.delete(requestId);
        }
      }
      if (this.cancelled.has(requestId)) throw new AppError({ code: "AI_CANCELLED", message: "论文对比已取消。" });
      progressService.update({id:requestId,kind:"compare",stage:"分析已准备，模型正在比较原文证据"});
      const result = await this.provider.comparePapers({ paper_ids: papers.map((paper) => paper.id), request_id: requestId });
      progressService.finish(requestId,"completed","深度对比与证据校验完成");
      return { result, requestId };
    } catch (error) { const appError=normalizeAppError(error); progressService.finish(requestId,appError.code === "AI_CANCELLED" ? "cancelled" : "failed",appError.message); throw error; }
    finally { this.preparing.delete(requestId); this.cancelled.delete(requestId); }
  }

  async cancel(requestId: string) {
    this.cancelled.add(requestId);
    const preparing = this.preparing.get(requestId);
    if (preparing) await this.analysisService?.cancel(preparing);
    await this.provider.cancel(requestId);
  }
}
