import type { Paper } from "../../domain/paper/Paper";
import { ExternalIcon, FileIcon, MoreIcon, SparkIcon } from "../../components/Icons";
import { formatAuthors, formatNumber } from "../../utils/formatters";

interface Props {
  paper?: Paper;
  analyzing: boolean;
  onAnalyze(): void;
  onOpen(): void;
  onDelete(): void;
}

function AnalysisContent({ paper }: { paper: Paper }) {
  const result = paper.analysis;
  if (!result) {
    return (
      <div className="analysis-empty">
        <div className="analysis-empty-icon"><SparkIcon/></div>
        <strong>No AI analysis yet</strong>
        <p>Run a private analysis using the selected local provider. Only a bounded paper excerpt is sent to LM Studio.</p>
      </div>
    );
  }
  return (
    <div className="analysis-sections">
      {result.ai_summary && <section className="analysis-summary"><span>AI summary</span><p>{result.ai_summary}</p></section>}
      {result.research_problem && <section><h3>Research Problem</h3><p>{result.research_problem}</p></section>}
      {result.core_method && <section><h3>Core Method</h3><p>{result.core_method}</p></section>}
      {!!result.key_contributions.length && <section><h3>Key Contributions</h3><ol>{result.key_contributions.map((item, index) => <li key={index}>{item}</li>)}</ol></section>}
      {result.main_results && <section><h3>Main Results</h3><p>{result.main_results}</p></section>}
      {result.limitations && <section><h3>Limitations</h3><p>{result.limitations}</p></section>}
      {!!result.tags.length && <section><h3>Keywords / Tags</h3><div className="detail-tags">{result.tags.map((tag) => <span key={tag}>{tag}</span>)}</div></section>}
    </div>
  );
}

export function PaperDetail({ paper, analyzing, onAnalyze, onOpen, onDelete }: Props) {
  if (!paper) {
    return <section className="detail-empty"><div className="empty-document"><FileIcon/></div><strong>Select a paper</strong><span>Metadata and local AI analysis will appear here.</span></section>;
  }
  return (
    <section className="paper-detail">
      <header className="detail-header">
        <div className="detail-title-block"><span className="document-type">PDF</span><h1>{paper.title}</h1><p>{formatAuthors(paper.authors)}{paper.year ? ` · ${paper.year}` : ""}</p></div>
        <div className="detail-actions">
          <button className="button secondary" onClick={onOpen}><ExternalIcon/>Open PDF</button>
          <button className="button primary" onClick={onAnalyze} disabled={analyzing}>{analyzing ? <span className="mini-spinner light"/> : <SparkIcon/>}{analyzing ? "Analyzing…" : paper.analysis ? "Analyze again" : "AI Analyze"}</button>
          <div className="menu-wrap"><button className="icon-button" aria-label="More paper actions"><MoreIcon/></button><button className="delete-action" onClick={onDelete}>Delete from library</button></div>
        </div>
      </header>
      <div className="detail-meta">
        <div><span>File</span><strong title={paper.file_path}>{paper.file_name}</strong></div>
        <div><span>Local path</span><strong title={paper.file_path}>{paper.file_path}</strong></div>
        {paper.pdf_diagnostics && <div><span>Extracted</span><strong>{formatNumber(paper.pdf_diagnostics.extracted_character_count)} characters · {paper.pdf_diagnostics.page_count} pages</strong></div>}
      </div>
      <div className="detail-scroll">
        {paper.abstract && <section className="overview-section"><h2>Overview</h2><p>{paper.abstract}</p></section>}
        <div className="section-title-row"><h2>AI Analysis</h2>{paper.analysis && <span className="analysis-version">Schema v{paper.analysis.schema_version}</span>}</div>
        <AnalysisContent paper={paper}/>
      </div>
    </section>
  );
}

