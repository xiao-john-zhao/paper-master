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
      ai_summary: "A concise overview",
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
      ai_summary: "summary",
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
      ai_summary: "summary",
    };

    expect(paperAnalysisSchema.safeParse(base).success).toBe(true);
    expect(paperAnalysisSchema.safeParse({ ...base, research_scenario: undefined }).success).toBe(false);
    expect(paperAnalysisSchema.safeParse({ ...base, key_innovations: [] }).success).toBe(false);
  });

  it("validates comparison output independently", () => {
    expect(paperComparisonSchema.safeParse({
      schema_version: "2",
      common_points: ["Common"],
      key_differences: ["Different"],
      method_comparison: "Methods compared",
      contribution_comparison: "Contributions compared",
      problem_comparison: "Problems compared",
      scenario_comparison: "Scenarios compared",
      results_comparison: "Results compared",
      limitations_comparison: "Limitations compared",
      evidence: [{ paper_number: 1, quote: "A sufficiently long source evidence quote." }],
      summary: "Summary",
    }).success).toBe(true);
  });
});
