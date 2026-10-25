import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { Sidebar, type AppView } from "../components/Sidebar";
import { Toast, type ToastMessage } from "../components/Toast";
import type { LibraryFilter, Paper } from "../domain/paper/Paper";
import { LibraryToolbar } from "../features/library/LibraryToolbar";
import { PaperList } from "../features/library/PaperList";
import { PaperDetail } from "../features/paper-detail/PaperDetail";
import { ComparisonDialog } from "../features/comparison/ComparisonDialog";
import type { PaperComparisonResult } from "../domain/ai/types";
import { DiagnosticsPage } from "../pages/DiagnosticsPage";
import { SettingsPage } from "../pages/SettingsPage";
import { createApplicationServices } from "../services/ApplicationServices";
import { createOperationId } from "../services/PaperService";
import type { AppSettings } from "../settings/types";
import { normalizeAppError } from "../types/errors";
import { uiLogger } from "../diagnostics/uiLogger";
import { ResearchPage } from "../pages/ResearchPage";
import type { AuthorContext } from "../domain/paper/AuthorContext";
import { TaskCenter,InlineTaskProgress } from "../components/TaskProgress";
import { progressService,type TaskUpdate } from "../services/ProgressService";
import { RelationshipGraphPage } from "../pages/RelationshipGraphPage";
import { LibraryCacheService } from "../services/LibraryCacheService";

const services = createApplicationServices();
const libraryCache = new LibraryCacheService();

export function App() {
  const [view, setView] = useState<AppView>("library");
  const [filter, setFilter] = useState<LibraryFilter>("all");
  const [tag, setTag] = useState<string>();
  const [search, setSearch] = useState("");
  const [papers, setPapers] = useState<Paper[]>([]);
  const [selectedId, setSelectedId] = useState<string>();
  const [compareIds, setCompareIds] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [cacheWarning, setCacheWarning] = useState("");
  const [onlineMode, setOnlineMode] = useState(false);
  const [authorContext, setAuthorContext] = useState<AuthorContext>();
  const [authorError, setAuthorError] = useState<string>();
  const [analyzingId, setAnalyzingId] = useState<string>();
  const [activeRequestId, setActiveRequestId] = useState<string>();
  const [comparisonOpen, setComparisonOpen] = useState(false);
  const [comparisonResult, setComparisonResult] = useState<PaperComparisonResult>();
  const [comparisonPapers, setComparisonPapers] = useState<Paper[]>([]);
  const [comparisonError, setComparisonError] = useState<string>();
  const [comparisonRequestId, setComparisonRequestId] = useState<string>();
  const [comparing, setComparing] = useState(false);
  const [importRequestId,setImportRequestId] = useState<string>();
  const [toast, setToast] = useState<ToastMessage>();
  const toastTimer = useRef<number | undefined>(undefined);
  const currentComparison = useRef<string | undefined>(undefined);

  const notify = useCallback((tone: ToastMessage["tone"], title: string, message?: string) => {
    window.clearTimeout(toastTimer.current);
    setToast({ id: Date.now(), tone, title, message });
    toastTimer.current = window.setTimeout(() => setToast(undefined), 4500);
  }, []);

  const loadPapers = useCallback(async (quiet = false) => {
    if (!quiet) setLoading(true);
    try {
      const result = await services.paperService.list({ text: search, tag, filter });
      setPapers(result);
      if (services.tauri) void libraryCache.status().then(status => setCacheWarning(status.last_error || (status.unreadable ? `${status.unreadable} 份论文缓存需要核对，请打开 Diagnostics 查看。` : ""))).catch(() => undefined);
      setSelectedId((current) => result.some((paper) => paper.id === current) ? current : result[0]?.id);
    } catch (error) {
      const appError = normalizeAppError(error);
      notify("error", "Library could not be loaded", appError.message);
    } finally { if (!quiet) setLoading(false); }
  }, [filter, notify, search, tag]);

  useEffect(() => { const timer = window.setTimeout(() => void loadPapers(), search ? 160 : 0); return () => window.clearTimeout(timer); }, [loadPapers, search]);
  useEffect(() => () => window.clearTimeout(toastTimer.current), []);
  useEffect(() => {
    if (!services.tauri) return;
    let disposed=false; let cleanup: (() => void) | undefined;
    void listen<TaskUpdate>("task-progress", event => { if (!disposed) progressService.update(event.payload); }).then(unlisten => { if (disposed) unlisten(); else cleanup=unlisten; });
    return () => { disposed=true; cleanup?.(); };
  },[]);
  useEffect(() => { services.settingsStorage.get().then((settings) => { setOnlineMode(settings.privacy_mode === "codex_online"); settingsSaved(settings); }).catch(() => undefined); }, []);
  useEffect(() => {
    if (!services.tauri) return;
    let disposed = false;
    const cleanup: Array<() => void> = [];
    for (const [event, handler] of [
      ["research-download", () => { void loadPapers(true); notify("success", "学校 PDF 下载完成，已处理导入任务"); }],
      ["research-download-error", (message: string) => notify("error", "学校 PDF 下载未完成", message)],
    ] as const) {
      void listen<string>(event, (message) => { if (!disposed) handler(message.payload); }).then((unlisten) => { if (disposed) unlisten(); else cleanup.push(unlisten); });
    }
    return () => { disposed = true; cleanup.forEach((unlisten) => unlisten()); };
  }, [loadPapers, notify]);
  useEffect(() => {
    if (!services.tauri) return;
    const timer = window.setInterval(() => void loadPapers(true), 2500);
    return () => window.clearInterval(timer);
  }, [loadPapers]);

  const selected = papers.find((paper) => paper.id === selectedId);
  useEffect(() => {
    let active = true;
    setAuthorContext(undefined); setAuthorError(undefined);
    if (selectedId && !services.tauri) setAuthorError("作者关系和原文证据在桌面版提供。");
    if (selectedId && services.tauri) void services.paperService.authorContext(selectedId).then((context) => { if (active) setAuthorContext(context); }).catch((error) => { if (active) setAuthorError(normalizeAppError(error).message); });
    return () => { active = false; };
  }, [selectedId, papers.length, selected?.authors.join("|")]);
  const tags = useMemo(() => {
    const counts = new Map<string, number>();
    papers.forEach((paper) => paper.tags.forEach((paperTag) => counts.set(paperTag, (counts.get(paperTag) ?? 0) + 1)));
    return [...counts].map(([name, count]) => ({ name, count })).sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
  }, [papers]);

  function toggleCompare(id: string) {
    setCompareIds((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else if (next.size < 8) next.add(id);
      else notify("info", "最多选择 8 篇论文", "请先取消一篇，再添加其他论文。 ");
      return next;
    });
  }

  async function importPapers(folder: boolean) {
    if (!services.tauri) { notify("info", "Import is available in the desktop app", "Browser preview uses stable demo papers."); return; }
    try {
      const selection = await open(folder ? { directory: true, multiple: false } : { multiple: true, filters: [{ name: "PDF documents", extensions: ["pdf"] }] });
      if (!selection) return;
      const paths = Array.isArray(selection) ? selection : [selection];
      const operationId = createOperationId("import");
      setImportRequestId(operationId);
      uiLogger.info("import_papers", "PDF import requested", { requestId: operationId });
      const result = await services.paperService.importPaths(paths, operationId);
      await loadPapers();
      const notes = [
        result.imported.some((paper) => paper.analysis_status === "queued") ? "已加入本地 AI 分析队列，完成后自动保存概括。" : "",
        result.duplicates.length ? `${result.duplicates.length} duplicate path${result.duplicates.length === 1 ? "" : "s"} skipped.` : "",
        result.failed.length ? `${result.failed.length} file${result.failed.length === 1 ? "" : "s"} could not be imported.` : "",
      ].filter(Boolean).join(" ");
      if (!result.imported.length && result.failed.length) notify("error", "No papers were imported", result.failed[0].message);
      else notify(result.failed.length ? "info" : "success", `${result.imported.length} paper${result.imported.length === 1 ? "" : "s"} imported`, notes || undefined);
    } catch (error) { const appError = normalizeAppError(error); notify("error", "Import failed", appError.message); }
  }

  async function analyzeSelected() {
    if (!selected || analyzingId || ["queued", "analyzing"].includes(selected.analysis_status)) return;
    const requestId = createOperationId("analysis");
    uiLogger.info("analyze_paper", "AI analysis requested", { requestId });
    setAnalyzingId(selected.id);
    setActiveRequestId(requestId);
    setPapers((items) => items.map((paper) => paper.id === selected.id ? { ...paper, analysis_status: "analyzing" } : paper));
    try {
      const result = await services.analysisService.analyze(selected, requestId);
      setPapers((items) => items.map((paper) => paper.id === result.paper.id ? result.paper : paper));
      notify("success", "Analysis complete", `Request ${result.requestId}`);
    } catch (error) {
      const appError = normalizeAppError(error);
      setPapers((items) => items.map((paper) => paper.id === selected.id ? { ...paper, analysis_status: "failed", analysis_error: appError.message } : paper));
      notify("error", "AI analysis failed", appError.message);
    } finally { setAnalyzingId(undefined); setActiveRequestId(undefined); }
  }

  async function cancelAnalysis() {
    if (!selected) return;
    try {
      if (activeRequestId && analyzingId === selected.id) await services.analysisService.cancel(activeRequestId);
      else await services.analysisService.cancelAutomatic(selected.id);
      await loadPapers(true);
      notify("info", "正在取消分析");
    }
    catch (error) { notify("error", "Analysis could not be cancelled", normalizeAppError(error).message); }
  }

  async function compareSelected() {
    if (compareIds.size < 2 || compareIds.size > 8) return;
    if (comparing) return;
    setComparing(true);
    const requestId = createOperationId("compare");
    currentComparison.current = requestId;
    uiLogger.info("compare_papers", "Paper comparison requested", { requestId });
    setComparisonResult(undefined); setComparisonError(undefined); setComparisonRequestId(requestId);
    try {
      // A search/filter can hide selected papers; resolve every selected ID from the repository.
      const resolved = await Promise.all([...compareIds].map((id) => services.paperService.get(id)));
      if (resolved.some((paper) => !paper)) throw new Error("部分已选论文已被移除，请重新选择。");
      const selectedPapers = resolved as Paper[];
      setComparisonPapers(selectedPapers); setComparisonOpen(true);
      const response = await services.comparisonService.compare(selectedPapers, requestId);
      if (currentComparison.current === requestId) setComparisonResult(response.result);
    } catch (error) {
      if (currentComparison.current === requestId) {
        const message = normalizeAppError(error).message;
        setComparisonError(message);
        notify("error", "论文对比未完成", message);
      }
    } finally { if (currentComparison.current === requestId) setComparing(false); }
  }

  async function cancelComparison() {
    currentComparison.current = undefined;
    if (comparisonRequestId && comparing) await services.comparisonService.cancel(comparisonRequestId).catch(() => undefined);
    setComparing(false); setComparisonOpen(false);
  }

  async function openSelected() {
    if (!selected) return;
    if (!services.tauri) { notify("info", "Open PDF is available in the desktop app", selected.file_path); return; }
    try { await services.paperService.openPdf(selected.id); }
    catch (error) { notify("error", "PDF could not be opened", normalizeAppError(error).message); }
  }

  async function relinkSelected() {
    if (!selected || !services.tauri) return;
    try {
      const path = await open({ multiple: false, filters: [{ name: "原始 PDF", extensions: ["pdf"] }] });
      if (!path || Array.isArray(path)) return;
      await services.paperService.relinkPdf(selected.id, path); await loadPapers(true);
      notify("success", "PDF 路径已更新");
    } catch (error) { notify("error", "重新关联失败", normalizeAppError(error).message); }
  }

  async function deleteSelected() {
    if (!selected || !window.confirm(`Remove “${selected.title}” from the local library?\n\nThe original PDF will not be deleted.`)) return;
    try { await services.paperService.remove(selected.id); await loadPapers(); notify("success", "Paper removed from library"); }
    catch (error) { notify("error", "Paper could not be removed", normalizeAppError(error).message); }
  }

  function settingsSaved(settings: AppSettings) {
    setOnlineMode(settings.privacy_mode === "codex_online");
    services.analysisService.setProvider(services.providers[settings.ai_provider === "codex" && settings.privacy_mode === "local_only" ? "lmstudio" : settings.ai_provider]);
    services.comparisonService.setProvider(services.providers[settings.ai_provider === "codex" && settings.privacy_mode === "local_only" ? "lmstudio" : settings.ai_provider]);
  }

  return (
    <div className="app-shell">
      <Sidebar view={view} filter={filter} tags={tags} activeTag={tag} onlineMode={onlineMode} onView={setView} onFilter={setFilter} onTag={setTag}/>
      {view === "library" && <div className="library-layout">
        <section className="library-column">
          <LibraryToolbar search={search} compareCount={compareIds.size} loading={loading} onSearch={setSearch} onImportPdf={() => void importPapers(false)} onImportFolder={() => void importPapers(true)} onCompare={() => void compareSelected()}/>
          {cacheWarning && <div className="library-import-progress" role="alert"><p>{cacheWarning}</p><button className="button secondary" onClick={() => setView("diagnostics")}>查看论文缓存</button></div>}
          {importRequestId && <div className="library-import-progress"><InlineTaskProgress id={importRequestId}/></div>}
          {(filter !== "all" || tag) && <div className="active-filter"><span>Showing {tag ? `tag: ${tag}` : filter === "recent" ? "recently added" : "AI analyzed"}</span><button onClick={() => { setTag(undefined); setFilter("all"); }}>Clear</button></div>}
          <PaperList papers={papers} selectedId={selectedId} compareIds={compareIds} loading={loading} onSelect={setSelectedId} onToggleCompare={toggleCompare}/>
          <footer className="library-count">{papers.length} paper{papers.length === 1 ? "" : "s"}</footer>
        </section>
        <PaperDetail onSaveMetadata={async metadata=>{if(selected){await services.paperService.editMetadata(selected.id,metadata);await loadPapers(true);}}} versionsService={services.analysisVersionsService} onChanged={() => void loadPapers(true)} requestId={analyzingId === selected?.id ? activeRequestId : undefined} paper={selected} authorContext={authorContext} authorError={authorError} onSelectRelated={(id) => { setSearch(""); setTag(undefined); setFilter("all"); setSelectedId(id); }} onRelink={() => void relinkSelected()} analyzing={!!selected && (analyzingId === selected.id || ["queued", "analyzing"].includes(selected.analysis_status))} onAnalyze={() => void analyzeSelected()} onCancel={() => void cancelAnalysis()} onOpen={() => void openSelected()} onDelete={() => void deleteSelected()}/>
      </div>}
      {view === "settings" && <SettingsPage storage={services.settingsStorage} providers={services.providers} onSaved={settingsSaved} onNotify={notify}/>} 
      {view === "diagnostics" && <DiagnosticsPage onNotify={notify}/>} 
      {view === "research" && <ResearchPage service={services.researchService} paperService={services.paperService} initialReferenceId={selectedId} desktop={services.tauri} onImported={() => void loadPapers(true)} onSettings={() => setView("settings")}/>}
      {view === "graph" && <RelationshipGraphPage service={services.graphService} onPaper={(id)=>{setSearch("");setTag(undefined);setFilter("all");setSelectedId(id);setView("library");}}/>}
      {comparisonOpen && comparisonPapers.length >= 2 && <ComparisonDialog onSave={async result=>{if(comparisonRequestId)setComparisonResult(await services.comparisonService.saveEdited(comparisonRequestId,result));}} papers={comparisonPapers} result={comparisonResult} loading={comparing} error={comparisonError} requestId={comparisonRequestId} onClose={() => setComparisonOpen(false)} onCancel={() => void cancelComparison()} onRetry={() => void compareSelected()}/>}
      <TaskCenter/>
      <Toast toast={toast} onClose={() => setToast(undefined)}/>
    </div>
  );
}
