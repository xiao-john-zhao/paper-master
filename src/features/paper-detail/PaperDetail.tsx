import type { Paper } from "../../domain/paper/Paper";
import { ExternalIcon, FileIcon, MoreIcon, SparkIcon } from "../../components/Icons";
import { formatAuthors, formatNumber } from "../../utils/formatters";

interface Props {
  paper?: Paper;
  analyzing: boolean;
  onAnalyze(): void;
  onCancel(): void;
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
  const modules = [
    { key: "problem", label: "问题", value: result.research_problem },
    { key: "scenario", label: "场景", value: result.research_scenario },
    { key: "method", label: "方法", value: result.core_method },
    { key: "innovation", label: "创新", values: result.key_innovations },
    { key: "contribution", label: "贡献", values: result.key_contributions },
  ].filter((module) => module.value || module.values?.length);
  return (
    <div className="analysis-sections">
      {result.ai_summary && <section className="analysis-summary"><span>一句话概览</span><p>{result.ai_summary}</p></section>}
      <div className="summary-framework" aria-label="问题-场景-方法-创新-贡献">
        {modules.map((module, moduleIndex) => (
          <section className="summary-module" key={module.key}>
            <div className="summary-module-heading"><span>{moduleIndex + 1}</span><h3>{module.label}</h3></div>
            {module.value && <p>{module.value}</p>}
            {!!module.values?.length && <ol>{module.values.map((item, index) => <li key={index}>{item}</li>)}</ol>}
          </section>
        ))}
      </div>
      {result.main_results && <section className="analysis-supplement"><h3>主要结果</h3><p>{result.main_results}</p></section>}
      {result.limitations && <section className="analysis-supplement"><h3>局限</h3><p>{result.limitations}</p></section>}
      {!!result.tags.length && <section className="analysis-supplement"><h3>关键词 / 标签</h3><div className="detail-tags">{result.tags.map((tag) => <span key={tag}>{tag}</span>)}</div></section>}
    </div>
  );
}

export function PaperDetail({ paper, analyzing, onAnalyze, onCancel, onOpen, onDelete }: Props) {
  if (!paper) {
    return <section className="detail-empty"><div className="empty-document"><FileIcon/></div><strong>Select a paper</strong><span>Metadata and local AI analysis will appear here.</span></section>;
  }
  return (
    <section className="paper-detail">
      <header className="detail-header">
        <div className="detail-title-block"><span className="document-type">PDF</span><h1>{paper.title}</h1><p>{formatAuthors(paper.authors)}{paper.year ? ` · ${paper.year}` : ""}</p></div>
        <div className="detail-actions">
          <button className="button secondary" onClick={onOpen}><ExternalIcon/>Open PDF</button>
          <button className={`button primary ${analyzing ? "cancel-analysis" : ""}`} onClick={analyzing ? onCancel : onAnalyze}>{analyzing ? <span className="mini-spinner light"/> : <SparkIcon/>}{analyzing ? "Cancel analysis" : paper.analysis ? "Analyze again" : "AI Analyze"}</button>
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
