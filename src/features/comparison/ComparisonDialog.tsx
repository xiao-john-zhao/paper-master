import type { PaperComparisonResult } from "../../domain/ai/types";
import type { Paper } from "../../domain/paper/Paper";
import { CompareIcon } from "../../components/Icons";

interface Props {
  papers: Paper[];
  result?: PaperComparisonResult;
  loading: boolean;
  error?: string;
  requestId?: string;
  onClose(): void;
  onCancel(): void;
  onRetry(): void;
}

export function ComparisonDialog({ papers, result, loading, error, requestId, onClose, onCancel, onRetry }: Props) {
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !loading) onClose(); }}>
      <section className="comparison-dialog" role="dialog" aria-modal="true" aria-labelledby="comparison-title">
        <header className="comparison-header">
          <div className="comparison-heading"><span className="comparison-icon"><CompareIcon/></span><div><h1 id="comparison-title">Multi-paper Comparison</h1><p>Comparing {papers.length} saved structured analyses · no full text resent</p></div></div>
          <button className="dialog-close" onClick={loading ? onCancel : onClose} aria-label={loading ? "Cancel comparison" : "Close comparison"}>×</button>
        </header>
        <div className="comparison-papers">{papers.map((paper, index) => <article key={paper.id}><span>Paper {index + 1}</span><strong title={paper.title}>{paper.title}</strong></article>)}</div>
        <div className="comparison-body">
          {loading && <div className="comparison-loading"><span className="large-spinner"/><strong>正在对比 {papers.length} 篇论文的结构化分析…</strong><p>本地模型处理可能需要一些时间，可随时取消且不会阻塞应用。</p>{requestId && <code>{requestId}</code>}<button className="button secondary" onClick={onCancel}>取消</button></div>}
          {!loading && error && <div className="comparison-error"><span>!</span><strong>Comparison failed</strong><p>{error}</p><button className="button primary" onClick={onRetry}>Try again</button></div>}
          {!loading && result && <div className="comparison-results">
            <section className="comparison-wide"><h2>共同点</h2><ul>{result.common_points.map((item, index) => <li key={index}>{item}</li>)}</ul></section>
            <section className="comparison-wide differences"><h2>关键差异</h2><ul>{result.key_differences.map((item, index) => <li key={index}>{item}</li>)}</ul></section>
            <section><h2>方法对比</h2><p>{result.method_comparison}</p></section>
            <section><h2>贡献对比</h2><p>{result.contribution_comparison}</p></section>
            <section className="comparison-wide overall"><h2>综合结论</h2><p>{result.summary}</p></section>
          </div>}
        </div>
        {!loading && result && <footer className="comparison-footer"><span>{requestId}</span><button className="button primary" onClick={onClose}>Done</button></footer>}
      </section>
    </div>
  );
}
