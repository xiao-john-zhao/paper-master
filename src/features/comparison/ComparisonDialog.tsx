import type { PaperComparisonResult } from "../../domain/ai/types";
import type { Paper } from "../../domain/paper/Paper";
import { CompareIcon } from "../../components/Icons";

interface Props {
  paperA: Paper;
  paperB: Paper;
  result?: PaperComparisonResult;
  loading: boolean;
  error?: string;
  requestId?: string;
  onClose(): void;
  onCancel(): void;
  onRetry(): void;
}

export function ComparisonDialog({ paperA, paperB, result, loading, error, requestId, onClose, onCancel, onRetry }: Props) {
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !loading) onClose(); }}>
      <section className="comparison-dialog" role="dialog" aria-modal="true" aria-labelledby="comparison-title">
        <header className="comparison-header">
          <div className="comparison-heading"><span className="comparison-icon"><CompareIcon/></span><div><h1 id="comparison-title">Paper Comparison</h1><p>Based on saved structured analyses · no full text resent</p></div></div>
          <button className="dialog-close" onClick={loading ? onCancel : onClose} aria-label={loading ? "Cancel comparison" : "Close comparison"}>×</button>
        </header>
        <div className="comparison-papers">
          <article><span>Paper A</span><strong>{paperA.title}</strong></article><div>vs</div><article><span>Paper B</span><strong>{paperB.title}</strong></article>
        </div>
        <div className="comparison-body">
          {loading && <div className="comparison-loading"><span className="large-spinner"/><strong>Comparing structured analyses…</strong><p>Local model processing can take a minute. You can cancel without blocking the application.</p>{requestId && <code>{requestId}</code>}<button className="button secondary" onClick={onCancel}>Cancel</button></div>}
          {!loading && error && <div className="comparison-error"><span>!</span><strong>Comparison failed</strong><p>{error}</p><button className="button primary" onClick={onRetry}>Try again</button></div>}
          {!loading && result && <div className="comparison-results">
            <section className="comparison-wide"><h2>Common Ground</h2><ul>{result.common_points.map((item, index) => <li key={index}>{item}</li>)}</ul></section>
            <section className="comparison-wide differences"><h2>Key Differences</h2><ul>{result.key_differences.map((item, index) => <li key={index}>{item}</li>)}</ul></section>
            <section><h2>Methods</h2><p>{result.method_comparison}</p></section>
            <section><h2>Contributions</h2><p>{result.contribution_comparison}</p></section>
            <section className="comparison-wide overall"><h2>Overall Comparison</h2><p>{result.summary}</p></section>
          </div>}
        </div>
        {!loading && result && <footer className="comparison-footer"><span>{requestId}</span><button className="button primary" onClick={onClose}>Done</button></footer>}
      </section>
    </div>
  );
}

