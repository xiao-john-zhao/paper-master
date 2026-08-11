import { describe, expect, it } from "vitest";
import { MockAIProvider } from "../src/providers/ai/MockAIProvider";
import { InMemoryPaperRepository } from "../src/repositories/InMemoryPaperRepository";
import { AnalysisService } from "../src/services/AnalysisService";
import { ComparisonService } from "../src/services/ComparisonService";
import { DEFAULT_SETTINGS } from "../src/settings/InMemorySettingsStorage";
import { AppError } from "../src/types/errors";

describe("application services", () => {
  it("uses a 600 second default LM Studio timeout", () => {
    expect(DEFAULT_SETTINGS.request_timeout_ms).toBe(600_000);
  });

  it("persists a provider analysis through the repository abstraction", async () => {
    const repository = new InMemoryPaperRepository();
    const paper = await repository.findById("demo-parser");
    expect(paper).not.toBeNull();

    const service = new AnalysisService(new MockAIProvider(), repository);
    const result = await service.analyze(paper!, "analysis-test-001");

    expect(result.requestId).toBe("analysis-test-001");
    expect(result.paper.analysis_status).toBe("analyzed");
    expect((await repository.findById("demo-parser"))?.analysis).toBeDefined();
  });

  it("refuses comparison when either saved analysis is missing", async () => {
    const repository = new InMemoryPaperRepository();
    const analyzed = await repository.findById("demo-section-aware");
    const pending = await repository.findById("demo-parser");
    const service = new ComparisonService(new MockAIProvider());

    await expect(service.compare([analyzed!, pending!], "compare-test-001"))
      .rejects.toMatchObject({ code: "PAPER_NOT_ANALYZED" } satisfies Partial<AppError>);
  });

  it("compares more than two analyzed papers with one request", async () => {
    const repository = new InMemoryPaperRepository();
    const first = await repository.findById("demo-section-aware");
    const second = await repository.findById("demo-retrieval");
    const third = { ...first!, id: "demo-third", title: "第三篇测试论文" };
    const service = new ComparisonService(new MockAIProvider());

    const response = await service.compare([first!, second!, third], "compare-test-003");
    expect(response.requestId).toBe("compare-test-003");
    expect(response.result.summary).toContain("论文");
  });
});
