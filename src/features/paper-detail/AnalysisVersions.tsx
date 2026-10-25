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
    {!draft&&<button className="button secondary" disabled={busy||!active} onClick={()=>{if(active){setEditing(active);setDraft(structuredClone(active.result));setError(undefined);}}}>编辑全部整理文字</button>}{!draft&&active?.edited&&<button className="button secondary" disabled={busy} onClick={()=>void perform(()=>service.restore(active),"已恢复最初生成文字")}>恢复生成文字</button>}</div>
    {active&&<small>来源：{analysisSource(active)} · {analysisTimeLabel(active)}：{analysisTime(active.created_at)}{active.edited ? ` · 最近修改：${analysisTime(active.updated_at)}（第 ${active.revision} 次保存）` : ""}。重新分析会新增结果，历史结果继续保留。</small>}
    {error&&<p role="alert" className="analysis-error">{error}</p>}{notice&&<p role="status">{notice}</p>}
    {draft&&editing&&<form className="analysis-editor" onSubmit={event=>{event.preventDefault();void perform(()=>service.edit(editing,draft),"编辑已保存，关键词已统一");}}>
      {fields.map(([key,label])=><label key={key}>{label}<textarea aria-label={label} rows={key==="ai_summary"||key==="main_results"?5:3} value={Array.isArray(draft[key])?(draft[key] as string[]).join("\n"):draft[key] as string ?? ""} onChange={e=>setDraft({...draft,[key]:arrays.has(key)?e.target.value.split("\n"):e.target.value})}/></label>)}
      <div className="analysis-edit-actions"><button type="submit" className="button primary" disabled={busy}>保存编辑</button><button type="button" className="button secondary" disabled={busy} onClick={()=>{setDraft(undefined);setEditing(undefined);}}>取消编辑</button></div>
    </form>}
  </section>;
}
