import type { PaperAnalysisResult } from "../../domain/paper/Paper";
import type { AnalysisRequest, CompareRequest, ConnectionResult, PaperComparisonResult } from "../../domain/ai/types";
import type { AIProvider } from "./AIProvider";

const MOCK_ANALYSIS: PaperAnalysisResult = {
  schema_version: "1",
  research_problem: "如何在保留关键科研语境的同时高效表示长篇研究文档？",
  core_method: "采用章节感知的表示流程，将摘要、引言、结论与少量有长度上限的证据段落组合起来。",
  key_contributions: [
    "提出可复现的章节感知文档表示策略。",
    "提出适用于本地语言模型的有界输入构建方法。",
    "设计区分文本提取故障与模型分析故障的评估协议。",
  ],
  main_results: "报告的评估表明，该方法在减少单篇论文处理文本量的同时提高了总结一致性。",
  limitations: "评估主要面向英文文本型 PDF，尚未覆盖扫描文档。",
  tags: ["文档分析", "本地大模型（LLM）", "科学自然语言处理（NLP）", "自动摘要"],
  ai_summary: "一个用于可靠本地学术论文分析的章节感知、有界上下文流程。",
};

function delay(milliseconds: number) {
  return new Promise((resolve) => globalThis.setTimeout(resolve, milliseconds));
}

export class MockAIProvider implements AIProvider {
  readonly id = "mock" as const;
  private cancelled = new Set<string>();

  async checkConnection(): Promise<ConnectionResult> {
    return { state: "connected", message: "Mock 模型已就绪。", latency_ms: 8, models: ["paper-master-mock-v1"], loaded_models: ["paper-master-mock-v1"] };
  }

  async getModels() {
    return ["paper-master-mock-v1"];
  }

  async loadModel() {
    return this.checkConnection();
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
      common_points: ["所选论文都关注学术文档的可靠处理。", "所选论文都强调可复现的本地工作流。"],
      key_differences: ["不同论文分别侧重章节感知输入构建、评估方法或证据检索。"],
      method_comparison: "各论文采用了不同但互补的章节选择、检索评分或学习排序策略。",
      contribution_comparison: "这些贡献分别覆盖数据准备、证据检索和评估框架，可在同一工作流中组合。",
      summary: "所选论文具有互补性：它们从输入构建、证据选择和效果评估等角度共同改进本地论文分析。",
    };
  }

  async cancel(requestId: string) {
    this.cancelled.add(requestId);
  }
}
