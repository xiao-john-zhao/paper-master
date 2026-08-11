import type { Paper } from "../domain/paper/Paper";
import type { PaperComparisonResult } from "../domain/ai/types";
import type { AIProvider } from "../providers/ai/AIProvider";
import { AppError } from "../types/errors";
import { createOperationId } from "./PaperService";

export class ComparisonService {
  constructor(private provider: AIProvider) {}

  setProvider(provider: AIProvider) { this.provider = provider; }

  async compare(paperA: Paper, paperB: Paper, requestId = createOperationId("compare")): Promise<{ result: PaperComparisonResult; requestId: string }> {
    if (!paperA.analysis) throw new AppError({ code: "PAPER_NOT_ANALYZED", message: `Analyze “${paperA.title}” before comparing papers.` });
    if (!paperB.analysis) throw new AppError({ code: "PAPER_NOT_ANALYZED", message: `Analyze “${paperB.title}” before comparing papers.` });
    const result = await this.provider.comparePapers({ paper_a_id: paperA.id, paper_b_id: paperB.id, request_id: requestId });
    return { result, requestId };
  }

  cancel(requestId: string) { return this.provider.cancel(requestId); }
}

