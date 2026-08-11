import { describe, expect, it } from "vitest";
import { KeywordSearchProvider } from "../src/providers/search/KeywordSearchProvider";
import { InMemoryPaperRepository } from "../src/repositories/InMemoryPaperRepository";

describe("paper repository and search", () => {
  it("filters analyzed papers and exact tags", async () => {
    const repository = new InMemoryPaperRepository();
    const analyzed = await repository.list({ filter: "analyzed" });
    const tagged = await repository.list({ tag: "privacy" });

    expect(analyzed.length).toBeGreaterThan(0);
    expect(analyzed.every((paper) => paper.analysis_status === "analyzed")).toBe(true);
    expect(tagged.map((paper) => paper.id)).toEqual(["demo-survey"]);
  });

  it("searches saved structured analysis fields", async () => {
    const repository = new InMemoryPaperRepository();
    const search = new KeywordSearchProvider(repository);
    const result = await search.search({ text: "deterministic text budget" });

    expect(result.map((paper) => paper.id)).toContain("demo-section-aware");
  });

  it("keeps an import operation id stable across the boundary", async () => {
    const repository = new InMemoryPaperRepository();
    const result = await repository.create(["paper.pdf"], "import-test-001");
    expect(result.operation_id).toBe("import-test-001");
  });
});
