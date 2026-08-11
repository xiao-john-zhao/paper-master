import type { AIProvider } from "../providers/ai/AIProvider";
import type { Paper } from "../domain/paper/Paper";
import type { PaperRepository } from "../repositories/PaperRepository";
import { createOperationId } from "./PaperService";

export class AnalysisService {
  constructor(private provider: AIProvider, private readonly repository: PaperRepository) {}

  setProvider(provider: AIProvider) { this.provider = provider; }

  async analyze(paper: Paper) {
    const requestId = createOperationId("analysis");
    const analysis = await this.provider.analyzePaper({ paper_id: paper.id, request_id: requestId });
    const updated: Paper = { ...paper, analysis, tags: analysis.tags, analysis_status: "analyzed", analysis_error: undefined, updated_at: new Date().toISOString() };
    return { paper: await this.repository.update(updated), requestId };
  }

  cancel(requestId: string) { return this.provider.cancel(requestId); }
}

