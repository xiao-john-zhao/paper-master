import type { Paper } from "../../domain/paper/Paper";
import { FileIcon } from "../../components/Icons";
import { formatAuthors, formatDate } from "../../utils/formatters";

interface Props {
  papers: Paper[];
  selectedId?: string;
  compareIds: Set<string>;
  loading: boolean;
  onSelect(id: string): void;
  onToggleCompare(id: string): void;
}

function Status({ paper }: { paper: Paper }) {
  if (paper.analysis_status === "analyzed") return <span className="analysis-status done"><span>✓</span> AI analyzed</span>;
  if (paper.analysis_status === "analyzing") return <span className="analysis-status running"><span className="mini-spinner"/> Analyzing</span>;
  if (paper.analysis_status === "queued") return <span className="analysis-status pending">等待本地分析</span>;
  if (paper.analysis_status === "failed") return <span className="analysis-status failed">Analysis failed</span>;
  return <span className="analysis-status pending">Not analyzed</span>;
}

export function PaperList({ papers, selectedId, compareIds, loading, onSelect, onToggleCompare }: Props) {
  if (!papers.length && !loading) {
    return <div className="list-empty"><FileIcon/><strong>No papers found</strong><span>Import a PDF or adjust the current search.</span></div>;
  }
  return (
    <div className="paper-list" role="listbox" aria-label="Papers">
      {papers.map((paper) => (
        <article key={paper.id} className={`paper-row ${selectedId === paper.id ? "selected" : ""}`} onClick={() => onSelect(paper.id)} role="option" aria-selected={selectedId === paper.id}>
          <label className="compare-check" title="Select for comparison" onClick={(event) => event.stopPropagation()}>
            <input type="checkbox" checked={compareIds.has(paper.id)} onChange={() => onToggleCompare(paper.id)}/><span/>
          </label>
          <div className="paper-row-body">
            <h3 title={paper.title}>{paper.title}</h3>
            <div className="paper-venue" title={paper.publication_venue}>{paper.publication_venue?.trim() || "刊出主体未识别"}</div>
            <div className="paper-byline" title={formatAuthors(paper.authors)}>{paper.authors.length ? formatAuthors(paper.authors) : "作者信息未识别"}</div>
            {paper.analysis_status === "analyzed" && paper.tags.length > 0 && <div className="row-tags">{paper.tags.slice(0, 3).map((tag) => <span key={tag}>{tag}</span>)}</div>}
            <div className="paper-row-footer"><Status paper={paper}/><time>{formatDate(paper.created_at)}</time></div>
          </div>
        </article>
      ))}
    </div>
  );
}
