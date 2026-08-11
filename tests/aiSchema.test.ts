import { describe, expect, it } from "vitest";
import { paperAnalysisSchema, paperComparisonSchema } from "../src/domain/ai/types";

describe("AI response schemas", () => {
  it("accepts the versioned structured analysis contract", () => {
    const result = paperAnalysisSchema.parse({
      schema_version: "2",
      research_problem: "A research problem",
      research_scenario: "A concrete application setting",
      core_method: "A reproducible method",
      key_innovations: ["One innovation"],
      key_contributions: ["One contribution"],
      main_results: "A measurable result",
      limitations: "A stated limitation",
      tags: ["local AI", "PDF", "research"],
    });

    expect(result.schema_version).toBe("2");
    expect(result.research_scenario).toBe("A concrete application setting");
    expect(result.key_innovations).toEqual(["One innovation"]);
    expect(result.tags).toHaveLength(3);
  });

  it("rejects incomplete or unbounded analysis output", () => {
    expect(() => paperAnalysisSchema.parse({ tags: [] })).toThrow();
    expect(() => paperAnalysisSchema.parse({
      schema_version: "2",
      research_problem: "problem",
      research_scenario: "scenario",
      core_method: "method",
      key_innovations: ["innovation"],
      key_contributions: ["contribution"],
      main_results: "result",
      limitations: "limitation",
      tags: Array.from({ length: 9 }, (_, index) => `tag-${index}`),
    })).toThrow();
  });

  it("requires separate problem, scenario, method, innovation, and contribution modules", () => {
    const base = {
      schema_version: "2",
      research_problem: "problem",
      research_scenario: "scenario",
      core_method: "method",
      key_innovations: ["innovation"],
      key_contributions: ["contribution"],
      main_results: "result",
      limitations: "limitation",
      tags: ["tag-1", "tag-2", "tag-3"],
    };

    expect(paperAnalysisSchema.safeParse(base).success).toBe(true);
    expect(paperAnalysisSchema.safeParse({ ...base, research_scenario: undefined }).success).toBe(false);
    expect(paperAnalysisSchema.safeParse({ ...base, key_innovations: [] }).success).toBe(false);
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
