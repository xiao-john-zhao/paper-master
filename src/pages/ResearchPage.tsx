import { useEffect, useRef, useState } from "react";
import { ResearchService, type ResearchResult } from "../services/ResearchService";
import { normalizeAppError } from "../types/errors";
import { SparkIcon } from "../components/Icons";
import type { Paper } from "../domain/paper/Paper";
import type { PaperService } from "../services/PaperService";
import { InlineTaskProgress } from "../components/TaskProgress";
import { taskId } from "../services/ProgressService";

interface Props { service: ResearchService; paperService: PaperService; initialReferenceId?: string; desktop: boolean; onImported(): void; onSettings(): void }

export function ResearchPage({ service, paperService, initialReferenceId, desktop, onImported, onSettings }: Props) {
  const [request, setRequest] = useState("");
  const [references, setReferences] = useState<Paper[]>([]);
  const [referenceId, setReferenceId] = useState(initialReferenceId ?? "");
  const [limit, setLimit] = useState(6);
  const [requestId, setRequestId] = useState<string>();
  const [progressId,setProgressId] = useState<string>();
  const [downloadProgressId,setDownloadProgressId] = useState<string>();
  const currentRequest = useRef<string | undefined>(undefined);
  const [result, setResult] = useState<ResearchResult>();
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [downloading, setDownloading] = useState<string>();
  useEffect(() => () => { if (currentRequest.current) void service.cancel(currentRequest.current).catch(() => undefined); currentRequest.current = undefined; }, [service]);
  useEffect(() => { let active = true; void paperService.list().then((papers) => { if (active) setReferences(papers); }).catch((error) => { if (active) setError(normalizeAppError(error).message); }); return () => { active = false; }; }, [paperService]);

  async function search() {
    const id = `research-${crypto.randomUUID()}`;
    currentRequest.current = id; setRequestId(id); setError(undefined); setNotice(undefined); setResult(undefined);
    setProgressId(id);
    try { const found = await service.search(request.trim(), limit, id, referenceId || undefined); if (currentRequest.current === id) setResult(found); }
    catch (error) { if (currentRequest.current === id) setError(normalizeAppError(error).message); }
    finally { if (currentRequest.current === id) { currentRequest.current = undefined; setRequestId(undefined); } }
  }
  async function cancel() {
    if (!requestId) return;
    try { await service.cancel(requestId); } catch (error) { setError(normalizeAppError(error).message); }
  }
  async function act(action: () => Promise<unknown>) {
    setError(undefined);
    try { await action(); } catch (error) { setError(normalizeAppError(error).message); }
  }
  return <main className="page-shell research-page">
    <header className="page-header"><div><h1>联网检索论文</h1><p>描述你要找的论文、作者或课题组，Codex 理解需求后检索 IEEE。</p></div><div className="research-actions"><button className="button secondary" disabled={!desktop} onClick={() => void act(() => service.openExternal())}>系统浏览器</button><button className="button secondary" disabled={!desktop} onClick={() => void act(() => service.openWebVpn())}>学校 IEEE / WebVPN</button></div></header>
    <section className="settings-panel research-search">
      <p>先在<button className="text-button" onClick={onSettings}>设置</button>中启用 Codex 联网授权。检索会发送你的需求及所选参考论文的题名和作者信息，不发送参考论文全文。</p>
      <form onSubmit={(event) => { event.preventDefault(); if (!requestId && desktop && request.trim()) void search(); }}>
        <label htmlFor="research-request">你想找什么论文？</label>
        <textarea id="research-request" rows={4} placeholder="例如：帮我找与这篇论文相同课题组、关于多无人机 ISAC 公平资源分配的相似文章，优先近三年期刊。也可以直接写作者名、论文题目或普通关键词。" value={request} maxLength={2000} disabled={!!requestId} onChange={(event) => setRequest(event.target.value)}/>
        <label htmlFor="research-reference">参考论文（可选，用于理解“这篇论文 / 该作者 / 同课题组”）</label>
        <select className="research-reference" id="research-reference" value={referenceId} disabled={!!requestId} onChange={(event) => setReferenceId(event.target.value)}><option value="">不使用本地参考论文</option>{references.map((paper) => <option value={paper.id} key={paper.id}>{paper.title} · {paper.authors[0] ?? "作者未识别"}</option>)}</select>
        <div className="research-search-row"><select aria-label="候选篇数" disabled={!!requestId} value={limit} onChange={(event) => setLimit(Number(event.target.value))}>{[3,6,10,12].map((value) => <option value={value} key={value}>{value} 篇</option>)}</select><button className="button primary" disabled={!desktop || !request.trim() || !!requestId}><SparkIcon/>{requestId ? "理解与检索中…" : "理解需求并检索"}</button>{requestId && <button type="button" className="button secondary" onClick={() => void cancel()}>取消</button>}</div>
      </form>
      <p className="research-note">学校订阅论文：打开内置 WebVPN 窗口，登录后在 IEEE 点击 PDF 下载，完成后尝试自动导入。该下载流程为试验功能；若仅打开阅读页，可将 PDF 保存后手动导入。浏览器会话可能需要独立登录。<button className="text-button" disabled={!desktop} onClick={() => void act(() => service.openCredentials())}>查看本机私有凭据 TXT</button></p>
      {!desktop && <p role="status">联网检索和文件下载需在桌面应用使用。</p>}
      {progressId&&<InlineTaskProgress id={progressId}/>}
      {downloadProgressId&&<InlineTaskProgress id={downloadProgressId}/>}
      {error && <p role="alert" className="analysis-error">{error}</p>}
      {notice && <p role="status">{notice}</p>}
    </section>
    {result && <><section className="settings-panel research-intent"><h2>我理解的检索需求</h2><p>{result.intent.summary}</p><dl><dt>研究主题</dt><dd>{result.intent.topic || "不限主题"}</dd><dt>目标作者</dt><dd>{result.intent.target_authors.join(", ") || "不限作者"}</dd><dt>关系条件</dt><dd>{{any:"主题/一般检索",shared_author:"要求同作者",potential_group:"要求作者或潜在同组证据"}[result.intent.relationship]}</dd>{result.intent.reference_title && <><dt>参考论文</dt><dd>{result.intent.reference_title}</dd></>}{(result.intent.year_from || result.intent.year_to) && <><dt>年份范围</dt><dd>{result.intent.year_from ?? "不限"} – {result.intent.year_to ?? "不限"}</dd></>}</dl>{result.intent.needs_clarification ? <p role="status" className="research-clarification">{result.intent.clarification_question} 请在上方补充需求或选择参考论文后重试。</p> : <details><summary>展开检索式</summary>{result.intent.search_queries.map((query,i) => <p key={i}>{query}</p>)}</details>}</section><p className="research-limitations">{result.limitations}</p><div className="research-results">{result.papers.map((paper) => <article className="settings-panel research-result" key={paper.ieee_url}>
      <div className="research-result-meta">{paper.venue || "出版物待核实"}{paper.year ? ` · ${paper.year}` : ""}{paper.citation_count !== null ? ` · 来源报告引用 ${paper.citation_count} 次` : " · 引用数未知"}</div>
      <h2>{paper.title}</h2><p>{paper.authors.join(", ") || "作者信息待核实"}</p><p className="research-quality">{paper.quality_reason}</p>
      <p className="research-relationship"><strong>{{topic_similar:"主题相似",shared_author:"同署名作者（身份待核实）",potential_group:"潜在同课题组（待核实）"}[paper.relationship]}</strong>{paper.matched_authors.length ? ` · 共同署名：${paper.matched_authors.join(", ")}` : ""}</p><p>{paper.relationship_evidence}</p>{paper.relationship_sources.length > 0 && <details><summary>作者 / 团队关系来源</summary>{paper.relationship_sources.map((url,i) => <p className="research-source-url" key={url}><button className="text-button" onClick={() => void act(() => service.openRelationshipSource(url))}>查看关系来源 {i+1}</button><small>{url}</small></p>)}</details>}
      {paper.doi && <small>DOI: {paper.doi}</small>}
      <div className="research-actions"><button className="button primary" disabled={!desktop} onClick={() => void act(() => service.openWebVpn(paper.ieee_url))}>学校权限下载（试验）</button><button className="button secondary" disabled={!desktop} onClick={() => void act(() => service.openExternal(paper.ieee_url))}>在系统浏览器查看来源</button>{paper.open_pdf_url && <button className="button secondary" disabled={!!downloading} onClick={() => void act(async () => {
        setDownloading(paper.ieee_url);
        const downloadId=taskId("download");setDownloadProgressId(downloadId);
        try { const summary = await service.download(paper.open_pdf_url,downloadId); onImported(); if (summary.failed.length) throw new Error(summary.failed[0].message); setNotice(`已下载并导入 ${summary.imported.length} 篇论文。`); }
        finally { setDownloading(undefined); }
      })}>{downloading === paper.ieee_url ? "下载中…" : "下载公开 PDF 并导入"}</button>}</div>
    </article>)}</div>{!result.intent.needs_clarification && !result.papers.length && <p className="research-limitations">未找到同时满足作者/团队、主题和时间条件的可核实候选。可修改上方需求后重试。</p>}</>}
  </main>;
}
