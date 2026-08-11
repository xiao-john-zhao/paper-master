import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
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
import { APP_CONFIG } from "../config/appConfig";
import { uiLogger } from "../diagnostics/uiLogger";

const services = createApplicationServices();

export function App() {
  const [view, setView] = useState<AppView>("library");
  const [filter, setFilter] = useState<LibraryFilter>("all");
  const [tag, setTag] = useState<string>();
  const [search, setSearch] = useState("");
  const [papers, setPapers] = useState<Paper[]>([]);
  const [selectedId, setSelectedId] = useState<string>();
  const [compareIds, setCompareIds] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [analyzingId, setAnalyzingId] = useState<string>();
  const [activeRequestId, setActiveRequestId] = useState<string>();
  const [comparisonOpen, setComparisonOpen] = useState(false);
  const [comparisonResult, setComparisonResult] = useState<PaperComparisonResult>();
  const [comparisonError, setComparisonError] = useState<string>();
  const [comparisonRequestId, setComparisonRequestId] = useState<string>();
  const [comparing, setComparing] = useState(false);
  const [toast, setToast] = useState<ToastMessage>();
  const toastTimer = useRef<number | undefined>(undefined);

  const notify = useCallback((tone: ToastMessage["tone"], title: string, message?: string) => {
    window.clearTimeout(toastTimer.current);
    setToast({ id: Date.now(), tone, title, message });
    toastTimer.current = window.setTimeout(() => setToast(undefined), 4500);
  }, []);

  const loadPapers = useCallback(async () => {
    setLoading(true);
    try {
      const result = await services.paperService.list({ text: search, tag, filter });
      setPapers(result);
      setSelectedId((current) => result.some((paper) => paper.id === current) ? current : result[0]?.id);
    } catch (error) {
      const appError = normalizeAppError(error);
      notify("error", "Library could not be loaded", appError.message);
    } finally { setLoading(false); }
  }, [filter, notify, search, tag]);

  useEffect(() => { const timer = window.setTimeout(() => void loadPapers(), search ? 160 : 0); return () => window.clearTimeout(timer); }, [loadPapers, search]);
  useEffect(() => () => window.clearTimeout(toastTimer.current), []);

  const selected = papers.find((paper) => paper.id === selectedId);
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
      uiLogger.info("import_papers", "PDF import requested", { requestId: operationId });
      const result = await services.paperService.importPaths(paths, operationId);
      await loadPapers();
      const notes = [
        result.duplicates.length ? `${result.duplicates.length} duplicate path${result.duplicates.length === 1 ? "" : "s"} skipped.` : "",
        result.failed.length ? `${result.failed.length} file${result.failed.length === 1 ? "" : "s"} could not be imported.` : "",
      ].filter(Boolean).join(" ");
      if (!result.imported.length && result.failed.length) notify("error", "No papers were imported", result.failed[0].message);
      else notify(result.failed.length ? "info" : "success", `${result.imported.length} paper${result.imported.length === 1 ? "" : "s"} imported`, notes || undefined);
    } catch (error) { const appError = normalizeAppError(error); notify("error", "Import failed", appError.message); }
  }

  async function analyzeSelected() {
    if (!selected) return;
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
    if (!activeRequestId) return;
    try { await services.analysisService.cancel(activeRequestId); notify("info", "Cancelling analysis", `Request ${activeRequestId}`); }
    catch (error) { notify("error", "Analysis could not be cancelled", normalizeAppError(error).message); }
  }

  async function compareSelected() {
    const selectedPapers = [...compareIds].map((id) => papers.find((paper) => paper.id === id)).filter((paper): paper is Paper => Boolean(paper));
    if (selectedPapers.length < 2 || selectedPapers.length > 8) return;
    const missing = selectedPapers.find((paper) => !paper.analysis);
    if (missing) { notify("info", "请先完成所选论文的分析", `《${missing.title}》还没有已保存的 AI 分析。`); return; }
    const requestId = createOperationId("compare");
    uiLogger.info("compare_papers", "Paper comparison requested", { requestId });
    setComparisonOpen(true); setComparing(true); setComparisonResult(undefined); setComparisonError(undefined); setComparisonRequestId(requestId);
    try {
      const response = await services.comparisonService.compare(selectedPapers, requestId);
      setComparisonResult(response.result);
    } catch (error) {
      setComparisonError(normalizeAppError(error).message);
    } finally { setComparing(false); }
  }

  async function cancelComparison() {
    if (comparisonRequestId && comparing) await services.comparisonService.cancel(comparisonRequestId).catch(() => undefined);
    setComparing(false); setComparisonOpen(false);
  }

  async function openSelected() {
    if (!selected) return;
    if (!services.tauri) { notify("info", "Open PDF is available in the desktop app", selected.file_path); return; }
    try { await openPath(selected.file_path); }
    catch (error) { notify("error", "PDF could not be opened", normalizeAppError(error).message); }
  }

  async function deleteSelected() {
    if (!selected || !window.confirm(`Remove “${selected.title}” from the local library?\n\nThe original PDF will not be deleted.`)) return;
    try { await services.paperService.remove(selected.id); await loadPapers(); notify("success", "Paper removed from library"); }
    catch (error) { notify("error", "Paper could not be removed", normalizeAppError(error).message); }
  }

  function settingsSaved(settings: AppSettings) {
    services.analysisService.setProvider(services.providers[settings.ai_provider]);
    services.comparisonService.setProvider(services.providers[settings.ai_provider]);
  }

  return (
    <div className="app-shell">
      <Sidebar view={view} filter={filter} tags={tags} activeTag={tag} developmentMode={APP_CONFIG.developmentMode || !services.tauri} onView={setView} onFilter={setFilter} onTag={setTag}/>
      {view === "library" && <div className="library-layout">
        <section className="library-column">
          <LibraryToolbar search={search} compareCount={compareIds.size} loading={loading} onSearch={setSearch} onImportPdf={() => void importPapers(false)} onImportFolder={() => void importPapers(true)} onCompare={() => void compareSelected()}/>
          {(filter !== "all" || tag) && <div className="active-filter"><span>Showing {tag ? `tag: ${tag}` : filter === "recent" ? "recently added" : "AI analyzed"}</span><button onClick={() => { setTag(undefined); setFilter("all"); }}>Clear</button></div>}
          <PaperList papers={papers} selectedId={selectedId} compareIds={compareIds} loading={loading} onSelect={setSelectedId} onToggleCompare={toggleCompare}/>
          <footer className="library-count">{papers.length} paper{papers.length === 1 ? "" : "s"}</footer>
        </section>
        <PaperDetail paper={selected} analyzing={analyzingId === selected?.id} onAnalyze={() => void analyzeSelected()} onCancel={() => void cancelAnalysis()} onOpen={() => void openSelected()} onDelete={() => void deleteSelected()}/>
      </div>}
      {view === "settings" && <SettingsPage storage={services.settingsStorage} providers={services.providers} onSaved={settingsSaved} onNotify={notify}/>} 
      {view === "diagnostics" && <DiagnosticsPage onNotify={notify}/>} 
      {comparisonOpen && (() => {
        const selectedPapers = [...compareIds].map((id) => papers.find((paper) => paper.id === id)).filter((paper): paper is Paper => Boolean(paper));
        return selectedPapers.length >= 2 ? <ComparisonDialog papers={selectedPapers} result={comparisonResult} loading={comparing} error={comparisonError} requestId={comparisonRequestId} onClose={() => setComparisonOpen(false)} onCancel={() => void cancelComparison()} onRetry={() => void compareSelected()}/> : null;
      })()}
      <Toast toast={toast} onClose={() => setToast(undefined)}/>
    </div>
  );
}
