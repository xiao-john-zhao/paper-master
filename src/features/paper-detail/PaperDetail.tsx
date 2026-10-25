import type { Paper } from "../../domain/paper/Paper";
import { ExternalIcon, FileIcon, MoreIcon, SparkIcon } from "../../components/Icons";
import { formatAuthors, formatNumber } from "../../utils/formatters";
import type { AuthorContext } from "../../domain/paper/AuthorContext";
import { InlineTaskProgress } from "../../components/TaskProgress";

import { AnalysisVersions } from "./AnalysisVersions";
import type { AnalysisVersionsService } from "../../services/AnalysisVersionsService";

import { MetadataEditor,type EditableMetadata } from "./MetadataEditor";

interface Props {
  onSaveMetadata(metadata:EditableMetadata):Promise<void>;
  versionsService: AnalysisVersionsService;
  onChanged(): void;
  paper?: Paper;
  analyzing: boolean;
  onAnalyze(): void;
  onCancel(): void;
  onOpen(): void;
  onDelete(): void;
  onRelink(): void;
  authorContext?: AuthorContext;
  authorError?: string;
  onSelectRelated(id: string): void;
  requestId?: string;
}

function AnalysisContent({ paper }: { paper: Paper }) {
  const result = paper.analysis;
  if (paper.analysis_provider === "extractive") {
    const usable = (text?: string) => text?.trim() && !/^(未可靠识别|本地摘录不推断|待阅读全文|待人工或模型|本地摘录未评估|未提取到可读文字)/.test(text.trim());
    return <div className="analysis-sections"><p className="analysis-pending-notice" role="status">尚未完成模型分析。以下仅为原文摘录，不作为模型总结；点击上方“AI 分析”可补齐。</p>
      {usable(result?.ai_summary) && <section><h3>摘要原文摘录</h3><p>{result?.ai_summary}</p></section>}
      {usable(result?.main_results) && <section><h3>结论原文摘录</h3><p>{result?.main_results}</p></section>}
      {!!paper.tags.length && <section><h3>原文关键词</h3><div className="detail-tags">{paper.tags.map(tag=><span key={tag}>{tag}</span>)}</div></section>}
    </div>;
  }
  if (!result) {
    return (
      <div className="analysis-empty">
        <div className="analysis-empty-icon"><SparkIcon/></div>
        <strong>No AI analysis yet</strong>
        <p>使用设置中选定的服务读取论文有界文本，生成中文问题、场景、方法、创新、贡献、结果和局限。Codex 模式需启用联网授权。</p>
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

export function PaperDetail({ paper, analyzing, onAnalyze, onCancel, onOpen, onDelete, onRelink, authorContext, authorError, onSelectRelated,requestId,versionsService,onChanged,onSaveMetadata }: Props) {
  if (!paper) {
    return <section className="detail-empty"><div className="empty-document"><FileIcon/></div><strong>Select a paper</strong><span>Metadata and local AI analysis will appear here.</span></section>;
  }
  return (
    <section className="paper-detail">
      <header className="detail-header">
        <div className="detail-title-block"><span className="document-type">PDF</span><h1>{paper.title}</h1><p>{formatAuthors(paper.authors)}{paper.year ? ` · ${paper.year}` : ""}</p></div>
        <div className="detail-actions">
          <button className="button secondary" onClick={onOpen}><ExternalIcon/>Open PDF</button>
          <button className="button secondary" onClick={onRelink}>重新关联 PDF</button>
          <button className={`button primary ${analyzing ? "cancel-analysis" : ""}`} onClick={analyzing ? onCancel : onAnalyze}>{analyzing ? <span className="mini-spinner light"/> : <SparkIcon/>}{analyzing ? "取消分析" : paper.analysis && paper.analysis_provider !== "extractive" ? "重新分析" : "AI 分析"}</button>
          <div className="menu-wrap"><button className="icon-button" aria-label="More paper actions"><MoreIcon/></button><button className="delete-action" onClick={onDelete}>Delete from library</button></div>
        </div>
      </header>
      <div className="detail-meta">
        <div><span>File</span><strong title={paper.file_path}>{paper.file_name}</strong></div>
        <div><span>Local path</span><strong title={paper.file_path}>{paper.file_path}</strong></div>
        {paper.pdf_diagnostics && <div><span>Extracted</span><strong>{formatNumber(paper.pdf_diagnostics.extracted_character_count)} characters · {paper.pdf_diagnostics.page_count} pages</strong></div>}
      </div>
      <div className="detail-scroll">
        <MetadataEditor key={`metadata-${paper.id}`} paper={paper} onSave={onSaveMetadata}/>
        <section className="author-information overview-section"><h2>作者与关联论文</h2>
          <dl><dt>全部作者</dt><dd>{(authorContext?.authors ?? paper.authors).join(", ") || "未识别"}</dd><dt>第一作者（署名顺序）</dt><dd>{authorContext?.first_author ?? paper.authors[0] ?? "未识别"}</dd><dt>通信作者</dt><dd>{authorContext?.corresponding_authors.length ? authorContext.corresponding_authors.join(", ") : "原文未识别到明确标注"}</dd></dl>
          {authorContext?.corresponding_evidence.map((quote, i) => <blockquote key={i}>{quote}</blockquote>)}
          {authorError && <p role="alert">{authorError}</p>}
          {authorContext ? <><h3>同作者 / 潜在同课题组论文（当前完整论文库）</h3>{authorContext.related_papers.length ? authorContext.related_papers.map((related) => <div className="related-paper" key={related.id}><button className="text-button" onClick={() => onSelectRelated(related.id)}>{related.title}</button><small>{related.relationship} · {related.shared_authors.join(", ")}</small><small>{related.evidence}</small></div>) : <p>当前库未找到同署名论文。课题组关系需结合通信作者和机构进一步核实。</p>}</> : !authorError && <p>正在读取作者关系…</p>}
        </section>
        {paper.abstract && <section className="overview-section"><h2>英文摘要 / Abstract</h2><p>{paper.abstract}</p></section>}
        <div className="section-title-row"><h2>论文整理与分析</h2>{paper.analysis && <span className="analysis-version">Schema v{paper.analysis.schema_version}</span>}</div>
        {analyzing && <InlineTaskProgress id={requestId ?? `analysis-auto-${paper.id}`} fallback="等待资源或正在分析，阶段信息将在服务响应时更新。"/>}
        {paper.analysis_status === "queued" && <p role="status">已加入本地分析队列，等待前面的任务完成。</p>}
        {paper.analysis_status === "analyzing" && <p role="status">所选 AI 服务正在读取和概括论文内容，完成后自动保存。</p>}
        {paper.analysis_error && <p role="alert" className="analysis-error">{paper.analysis_error}</p>}
        <AnalysisVersions key={`analysis-${paper.id}`} paper={paper} service={versionsService} onChanged={onChanged}/>
        <AnalysisContent paper={paper}/>
      </div>
    </section>
  );
}
