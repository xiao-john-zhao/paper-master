import type { ImportSummary, Paper, PaperQuery } from "../domain/paper/Paper";
import type { PaperRepository } from "./PaperRepository";

const now = new Date();
const day = 86_400_000;

const DEMO_PAPERS: Paper[] = [
  {
    id: "demo-section-aware",
    file_path: "C:\\Research\\section-aware-scientific-analysis.pdf",
    file_name: "section-aware-scientific-analysis.pdf",
    title: "Section-Aware Context Selection for Local Scientific Document Analysis",
    authors: ["Maya Chen", "Elias Novak", "Priya Raman"],
    year: 2025,
    abstract: "We study bounded context construction for reliable analysis of scientific documents with local language models.",
    tags: ["scientific NLP", "local LLM", "summarization"],
    analysis_status: "analyzed",
    analysis: {
      schema_version: "2",
      research_problem: "Local language models have limited context and can miss essential evidence when long papers are truncated without regard to document structure.",
      research_scenario: "Private analysis of long, text-based scientific PDFs with local language models on a researcher's computer.",
      core_method: "The authors detect major paper sections and assemble a bounded representation from the title, abstract, introduction, conclusion, and a small set of supporting passages.",
      key_innovations: [
        "Combining section priorities with a deterministic input budget instead of uniform truncation.",
        "Separating PDF extraction diagnostics from downstream model failures in one local workflow.",
      ],
      key_contributions: [
        "A section-aware input builder designed for local language models.",
        "A deterministic text budget that preserves high-value scientific context.",
        "Diagnostics that separate PDF extraction failures from downstream model failures.",
      ],
      main_results: "The reported evaluation shows more consistent structured summaries while processing less input text than uniform truncation baselines.",
      limitations: "The evaluation is limited to English text-based PDFs and a small family of local instruction models.",
      tags: ["scientific NLP", "local LLM", "summarization", "document parsing"],
      ai_summary: "A bounded, section-aware context strategy improves the reliability of local scientific-paper analysis.",
    },
    pdf_diagnostics: {
      page_count: 12,
      extracted_character_count: 48_291,
      detected_sections: ["abstract", "introduction", "conclusion"],
      parser_status: "success",
      warnings: [],
      fallback_used: false,
    },
    created_at: new Date(now.getTime() - day).toISOString(),
    updated_at: new Date(now.getTime() - day + 7_200_000).toISOString(),
  },
  {
    id: "demo-retrieval",
    file_path: "C:\\Research\\evidence-retrieval.pdf",
    file_name: "evidence-retrieval.pdf",
    title: "Evidence-Grounded Retrieval for Reproducible Literature Review",
    authors: ["Sofia Alvarez", "Daniel Brooks"],
    year: 2024,
    abstract: "A retrieval framework for evidence-grounded review of local scientific collections.",
    tags: ["information retrieval", "literature review"],
    analysis_status: "analyzed",
    analysis: {
      schema_version: "2",
      research_problem: "Literature review tools often produce claims without preserving a reproducible connection to supporting paper passages.",
      research_scenario: "Evidence-grounded literature review over a domain-specific local collection of scientific papers.",
      core_method: "A hybrid candidate generator and learned passage ranker return evidence with stable paper and section references.",
      key_innovations: ["Stable paper-and-section evidence references attached to ranked review passages."],
      key_contributions: ["An evidence-linked retrieval protocol.", "A reproducibility-oriented evaluation set."],
      main_results: "The approach improves passage recall and reviewer agreement over the compared keyword baseline.",
      limitations: "The collection is domain-specific and citation relationships are not modeled.",
      tags: ["information retrieval", "evidence grounding", "literature review"],
      ai_summary: "A retrieval workflow that keeps literature-review claims tied to reproducible supporting evidence.",
    },
    created_at: new Date(now.getTime() - day * 4).toISOString(),
    updated_at: new Date(now.getTime() - day * 3).toISOString(),
  },
  {
    id: "demo-parser",
    file_path: "C:\\Research\\robust-pdf-parsing.pdf",
    file_name: "robust-pdf-parsing.pdf",
    title: "Robust Section Detection in Heterogeneous Scientific PDFs",
    authors: ["Noah Williams", "Lin Wei", "Amara Okafor"],
    year: 2023,
    abstract: "Rule-based and statistical section detection for heterogeneous scientific PDFs.",
    tags: ["PDF parsing", "section detection"],
    analysis_status: "not_analyzed",
    created_at: new Date(now.getTime() - day * 8).toISOString(),
    updated_at: new Date(now.getTime() - day * 8).toISOString(),
  },
  {
    id: "demo-survey",
    file_path: "C:\\Research\\private-ai-tools-survey.pdf",
    file_name: "private-ai-tools-survey.pdf",
    title: "Private-by-Design AI Tools for Individual Researchers: A Survey",
    authors: ["Camille Martin"],
    year: 2025,
    tags: ["privacy", "research tools"],
    analysis_status: "not_analyzed",
    created_at: new Date(now.getTime() - day * 15).toISOString(),
    updated_at: new Date(now.getTime() - day * 15).toISOString(),
  },
];

function matches(paper: Paper, query: PaperQuery) {
  if (query.filter === "analyzed" && paper.analysis_status !== "analyzed") return false;
  if (query.filter === "recent" && Date.now() - new Date(paper.created_at).getTime() > day * 7) return false;
  if (query.tag && !paper.tags.some((tag) => tag.toLowerCase() === query.tag!.toLowerCase())) return false;
  const needle = query.text?.trim().toLowerCase();
  if (!needle) return true;
  return [paper.title, paper.authors.join(" "), paper.abstract ?? "", paper.analysis?.ai_summary ?? "", paper.analysis?.research_problem ?? "", paper.analysis?.research_scenario ?? "", paper.analysis?.core_method ?? "", paper.analysis?.key_innovations?.join(" ") ?? "", paper.analysis?.key_contributions.join(" ") ?? "", paper.tags.join(" ")]
    .join(" ").toLowerCase().includes(needle);
}

export class InMemoryPaperRepository implements PaperRepository {
  private papers = structuredClone(DEMO_PAPERS);

  async create(_paths: string[], operationId: string): Promise<ImportSummary> {
    return { operation_id: operationId, imported: [], duplicates: [], failed: [] };
  }
  async update(paper: Paper) {
    const index = this.papers.findIndex((item) => item.id === paper.id);
    if (index >= 0) this.papers[index] = structuredClone(paper);
    else this.papers.unshift(structuredClone(paper));
    return structuredClone(paper);
  }
  async delete(id: string) { this.papers = this.papers.filter((paper) => paper.id !== id); }
  async findById(id: string) { return structuredClone(this.papers.find((paper) => paper.id === id) ?? null); }
  async list(query: PaperQuery = {}) { return structuredClone(this.papers.filter((paper) => matches(paper, query))); }
  async search(query: PaperQuery) { return this.list(query); }
  async existsByPath(path: string) { return this.papers.some((paper) => paper.file_path === path); }
}
