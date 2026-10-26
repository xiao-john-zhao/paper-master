import { useEffect,useRef,useState } from "react";
import type { Paper,PaperAnalysisResult } from "../../domain/paper/Paper";
import { AnalysisVersionsService,type AnalysisVersion } from "../../services/AnalysisVersionsService";
import { normalizeAppError } from "../../types/errors";
import { analysisSource, analysisTime, analysisTimeLabel, analysisVersionLabel } from "../../domain/paper/AnalysisSource";

const fields = [
  ["ai_summary","概览"],["research_problem","研究问题"],["research_scenario","研究场景"],
  ["core_method","核心方法"],["key_innovations","创新点（每行一项）"],["key_contributions","贡献（每行一项）"],
  ["main_results","主要结果"],["limitations","局限"],["tags","关键词（每行一项，保存时统一同义词）"],
] as const;
const arrays = new Set(["key_innovations","key_contributions","tags"]);
export function AnalysisVersions({paper,service,onChanged}:{paper:Paper;service:AnalysisVersionsService;onChanged():void}) {
  const [versions,setVersions]=useState<AnalysisVersion[]>([]);
  const [draft,setDraft]=useState<PaperAnalysisResult>();
  const [editing,setEditing]=useState<AnalysisVersion>();
  const [busy,setBusy]=useState(false);const [error,setError]=useState<string>();const [notice,setNotice]=useState<string>();
  const generation=useRef(0);
  useEffect(()=>{setDraft(undefined);setEditing(undefined);setError(undefined);setNotice(undefined);},[paper.id]);
  useEffect(()=>{const current=++generation.current;void service.list(paper).then(value=>{if(generation.current===current)setVersions(value);}).catch(e=>{if(generation.current===current)setError(normalizeAppError(e).message);});return()=>{generation.current++;};},[paper.id,paper.updated_at,service]);
  const active=versions.find(v=>v.active);
  async function perform(action:()=>Promise<unknown>,message:string) {
    setBusy(true);setError(undefined);setNotice(undefined);
    try { await action();setDraft(undefined);setEditing(undefined);setVersions(await service.list(paper));onChanged();setNotice(message); }
    catch(e){setError(normalizeAppError(e).message);}finally{setBusy(false);}
  }
  return <section className="analysis-version-panel" aria-label="分析版本与编辑">
    <div className="analysis-version-toolbar"><label>查看整理结果<select aria-label="查看整理结果" disabled={busy||!!draft||!versions.length} value={active?.id ?? ""} onChange={e=>{const version=versions.find(v=>v.id===e.target.value);if(version)void perform(()=>service.select(version),"已切换整理结果");}}>
      {!versions.length&&<option value="">暂无保存结果</option>}
      {versions.map(v=><option key={v.id} value={v.id}>{analysisVersionLabel(v)}</option>)}
    </select></label>
    <button className="button secondary" disabled={busy||!!draft||!service.desktop} title={service.desktop?"从原文建立本地摘录":"本地摘录在桌面应用提供"} onClick={()=>void perform(()=>service.extract(paper),"本地原文摘录已保留")}>本地归档摘录</button>
    {!draft&&<button className="button secondary" disabled={busy||!active} onClick={()=>{if(active){setEditing(active);setDraft(structuredClone(active.result));setError(undefined);}}}>编辑 AI 分析内容</button>}{!draft&&active?.edited&&<button className="button secondary" disabled={busy} onClick={()=>void perform(()=>service.restore(active),"已恢复最初生成文字")}>恢复生成文字</button>}</div>
    {active&&<small>来源：{analysisSource(active)} · {analysisTimeLabel(active)}：{analysisTime(active.created_at)}{active.edited ? ` · 最近修改：${analysisTime(active.updated_at)}（第 ${active.revision} 次保存）` : ""}。重新分析会新增结果，历史结果继续保留。</small>}
    {active?.profile&&<small className="analysis-profile">本次成功参数：第 {active.profile.attempts} 次尝试 · 输入 {active.profile.input_characters} / {active.profile.input_budget} 字符 · 输出上限 {active.profile.output_tokens} token · 推理 {active.profile.reasoning_effort}{active.provider === "lmstudio" ? ` · 请求加载上下文 ${active.profile.requested_context_length} token（已加载模型的实际上下文以 LM Studio 为准）` : ""}。</small>}
    {active?.profile?.reading&&<details className="analysis-reading-report"><summary>处理方案与语义可信度复核：{active.profile.reading.strategy==="segmented"?"完整分段阅读 → 分层综合":"直接分析（有界材料）"}</summary>
      <p>整体语义基准：{active.profile.reading.baseline?.summary??"未生成"}</p>
      <p>语义复核：{active.profile.reading.audit?.reason??"无复核记录"}。此检查结合基准与有界原文证据，不是正确性概率。</p>
      {active.edited&&<p>以上复核针对最初生成内容，人工修改后的文字尚未重新进行模型复核。</p>}
      {active.profile.reading.audit?.quotes.map((quote,i)=><blockquote key={i}>{quote}</blockquote>)}
      {active.profile.reading.strategy==="segmented"&&<><p>已覆盖提取文字 {active.profile.reading.covered_characters} / {active.profile.reading.source_characters} 字符 · {active.profile.reading.parts.length} 段 · {active.profile.reading.consolidation_rounds} 轮综合。覆盖率对应可提取文字，不代表扫描图片或全部 PDF 页面均已识别。</p>
      {active.profile.reading.parts.map((part,i)=><details key={i}><summary>第 {i+1} 段 · 原文字符 {part.start}–{part.end} · 预算 {part.input_budget} · 推理 {part.reasoning_effort}</summary><p>{part.note.summary}</p>{part.note.claims.map((claim,j)=><div key={j}><p>{claim.finding}</p><blockquote>{claim.quote}</blockquote></div>)}</details>)}</>}
    </details>}
    {active&&<p className="analysis-edit-hint">可修改当前 AI 版本的全部字段；原始生成内容和其他模型版本会保留，保存后自动更新检索与论文缓存。</p>}
    {error&&<p role="alert" className="analysis-error">{error}</p>}{notice&&<p role="status">{notice}</p>}
    {draft&&editing&&<form className="analysis-editor" aria-label="编辑 AI 分析内容" onSubmit={event=>{event.preventDefault();void perform(()=>service.edit(editing,draft),"编辑已保存，关键词已统一");}}>
      {fields.map(([key,label])=><label key={key}>{label}<textarea aria-label={label} rows={key==="ai_summary"||key==="main_results"?5:3} value={Array.isArray(draft[key])?(draft[key] as string[]).join("\n"):draft[key] as string ?? ""} onChange={e=>setDraft({...draft,[key]:arrays.has(key)?e.target.value.split("\n"):e.target.value})}/></label>)}
      <div className="analysis-edit-actions"><button type="submit" className="button primary" disabled={busy}>保存编辑</button><button type="button" className="button secondary" disabled={busy} onClick={()=>{setDraft(undefined);setEditing(undefined);}}>取消编辑</button></div>
    </form>}
  </section>;
}
