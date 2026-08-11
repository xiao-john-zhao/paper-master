import type { Paper } from "../domain/paper/Paper";
import type { PaperComparisonResult } from "../domain/ai/types";
import type { AIProvider } from "../providers/ai/AIProvider";
import { AppError } from "../types/errors";
import { createOperationId } from "./PaperService";

export class ComparisonService {
  constructor(private provider: AIProvider) {}

  setProvider(provider: AIProvider) { this.provider = provider; }

  async compare(papers: Paper[], requestId = createOperationId("compare")): Promise<{ result: PaperComparisonResult; requestId: string }> {
    if (papers.length < 2 || papers.length > 8 || new Set(papers.map((paper) => paper.id)).size !== papers.length) {
      throw new AppError({ code: "PAPER_NOT_ANALYZED", message: "请选择 2–8 篇互不相同的论文进行对比。" });
    }
    const missing = papers.find((paper) => !paper.analysis);
    if (missing) throw new AppError({ code: "PAPER_NOT_ANALYZED", message: `请先分析《${missing.title}》，再执行多篇论文对比。` });
    const result = await this.provider.comparePapers({ paper_ids: papers.map((paper) => paper.id), request_id: requestId });
    return { result, requestId };
  }

  cancel(requestId: string) { return this.provider.cancel(requestId); }
}
