import { describe, expect, it } from "vitest";
import { paperAnalysisSchema, paperComparisonSchema } from "../src/domain/ai/types";

describe("AI response schemas", () => {
  it("accepts the versioned structured analysis contract", () => {
    const result = paperAnalysisSchema.parse({
      schema_version: "1",
      research_problem: "A research problem",
      core_method: "A reproducible method",
      key_contributions: ["One contribution"],
      main_results: "A measurable result",
      limitations: "A stated limitation",
      tags: ["local AI", "PDF", "research"],
    });

    expect(result.schema_version).toBe("1");
    expect(result.tags).toHaveLength(3);
  });

  it("rejects incomplete or unbounded analysis output", () => {
    expect(() => paperAnalysisSchema.parse({ tags: [] })).toThrow();
    expect(() => paperAnalysisSchema.parse({
      schema_version: "1",
      research_problem: "problem",
      core_method: "method",
      key_contributions: ["contribution"],
      main_results: "result",
      limitations: "limitation",
      tags: Array.from({ length: 9 }, (_, index) => `tag-${index}`),
    })).toThrow();
  });

  it("validates comparison output independently", () => {
    expect(paperComparisonSchema.safeParse({
      common_points: ["Common"],
      key_differences: ["Different"],
      method_comparison: "Methods compared",
      contribution_comparison: "Contributions compared",
      summary: "Summary",
    }).success).toBe(true);
  });
});
