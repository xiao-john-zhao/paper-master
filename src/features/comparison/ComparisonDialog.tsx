import { useState } from "react";
import type { PaperComparisonResult } from "../../domain/ai/types";
import type { Paper } from "../../domain/paper/Paper";
import { CompareIcon } from "../../components/Icons";
import { InlineTaskProgress } from "../../components/TaskProgress";

interface Props {
  papers: Paper[];
  result?: PaperComparisonResult;
  loading: boolean;
  error?: string;
  requestId?: string;
  onSave(result:PaperComparisonResult):Promise<void>;
  onClose(): void;
  onCancel(): void;
  onRetry(): void;
}

export function ComparisonDialog({ papers, result, loading, error, requestId, onClose, onCancel, onRetry,onSave }: Props) {
  const [draft,setDraft]=useState<PaperComparisonResult>();const [saving,setSaving]=useState(false);const [editError,setEditError]=useState<string>();
  const labels=[ ["common_points","共同点"],["key_differences","关键差异"],["problem_comparison","研究问题"],["scenario_comparison","场景与假设"],["method_comparison","方法对比"],["contribution_comparison","贡献对比"],["results_comparison","实验与结果"],["limitations_comparison","局限与适用条件"],["summary","综合结论"] ] as const;
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !loading) onClose(); }}>
      <section className="comparison-dialog" role="dialog" aria-modal="true" aria-labelledby="comparison-title">
        <header className="comparison-header">
          <div className="comparison-heading"><span className="comparison-icon"><CompareIcon/></span><div><h1 id="comparison-title">多篇论文深度对比</h1><p>{papers.length} 篇论文 · 本地模型结合内容分析与原文证据生成差异</p></div></div>
          <button className="dialog-close" onClick={loading ? onCancel : onClose} aria-label={loading ? "Cancel comparison" : "Close comparison"}>×</button>
        </header>
        <div className="comparison-papers">{papers.map((paper, index) => <article key={paper.id}><span>Paper {index + 1}</span><strong title={paper.title}>{paper.title}</strong></article>)}</div>
        <div className="comparison-body">
          {loading && <InlineTaskProgress id={requestId}/>}
          {loading && <div className="comparison-loading"><span className="large-spinner"/><strong>正在准备并深度对比 {papers.length} 篇论文…</strong><p>自动补齐尚未完成的内容分析，再结合原文证据比较研究差异。本地任务依次执行，可随时取消。</p>{requestId && <code>{requestId}</code>}<button className="button secondary" onClick={onCancel}>取消</button></div>}
          {!loading && error && <div className="comparison-error"><span>!</span><strong>Comparison failed</strong><p>{error}</p><button className="button primary" onClick={onRetry}>Try again</button></div>}
          {!loading && result && <div><button className="button secondary" onClick={()=>setDraft(draft?undefined:structuredClone(result))}>{draft?"取消编辑":"编辑对比文字"}</button>{editError&&<p role="alert">{editError}</p>}{draft&&<form className="analysis-editor" onSubmit={async e=>{e.preventDefault();setSaving(true);setEditError(undefined);try{await onSave(draft);setDraft(undefined);}catch(e){setEditError(String(e));}finally{setSaving(false);}}}>{labels.map(([key,label])=><label key={key}>{label}<textarea rows={4} value={Array.isArray(draft[key])?(draft[key] as string[]).join("\n"):draft[key] as string} onChange={e=>setDraft({...draft,[key]:Array.isArray(draft[key])?e.target.value.split("\n").filter(Boolean):e.target.value})}/></label>)}<button className="button primary" disabled={saving}>保存对比编辑</button></form>}</div>}
          {!loading && result && <div className="comparison-results">
            <section className="comparison-wide"><h2>共同点</h2><ul>{result.common_points.map((item, index) => <li key={index}>{item}</li>)}</ul></section>
            <section className="comparison-wide differences"><h2>关键差异</h2><ul>{result.key_differences.map((item, index) => <li key={index}>{item}</li>)}</ul></section>
            <section><h2>研究问题</h2><p>{result.problem_comparison}</p></section>
            <section><h2>场景与假设</h2><p>{result.scenario_comparison}</p></section>
            <section><h2>方法对比</h2><p>{result.method_comparison}</p></section>
            <section><h2>贡献对比</h2><p>{result.contribution_comparison}</p></section>
            <section><h2>实验与结果</h2><p>{result.results_comparison}</p></section>
            <section><h2>局限与适用条件</h2><p>{result.limitations_comparison}</p></section>
            {!!result.evidence.length && <section className="comparison-wide"><h2>原文证据</h2>{result.evidence.map((item, index) => <blockquote key={index}><strong>论文{item.paper_number} · {papers[item.paper_number - 1]?.title}</strong><p>{item.quote}</p></blockquote>)}</section>}
            <section className="comparison-wide overall"><h2>综合结论</h2><p>{result.summary}</p></section>
          </div>}
        </div>
        {!loading && result && <footer className="comparison-footer"><span>{requestId}</span><button className="button primary" onClick={onClose}>Done</button></footer>}
      </section>
    </div>
  );
}
