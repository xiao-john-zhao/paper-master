import { useEffect, useState } from "react";
import { invokeCommand, isTauriRuntime } from "../adapters/tauri/invokeCommand";
import type { DiagnosticSnapshot } from "../diagnostics/types";
import { ExternalIcon } from "../components/Icons";
import { LibraryCachePanel } from "../features/library/LibraryCachePanel";
import { APP_CONFIG } from "../config/appConfig";

const browserSnapshot: DiagnosticSnapshot = {
  application_version: APP_CONFIG.version, environment: "Browser preview", database_path: "Not available in browser preview", database_status: "Preview data", migration_version: 0,
  lm_studio_base_url: "http://127.0.0.1:1234/v1", lm_studio_status: "disconnected", current_model: "paper-master-mock-v1", paper_count: 4, analyzed_paper_count: 2,
  last_ai_request_status: "Mock analysis succeeded", last_import_status: "No import in this session", recent_errors: [], log_file_location: "Available in Tauri development mode",
};

export function DiagnosticsPage({ onNotify }: { onNotify(tone: "success" | "error" | "info", title: string, message?: string): void }) {
  const [snapshot, setSnapshot] = useState<DiagnosticSnapshot>(browserSnapshot);
  const [busy, setBusy] = useState(false);
  async function refresh() {
    setBusy(true);
    try { setSnapshot(isTauriRuntime() ? await invokeCommand<DiagnosticSnapshot>("get_diagnostics") : browserSnapshot); }
    catch (error) { onNotify("error", "Diagnostics unavailable", error instanceof Error ? error.message : String(error)); }
    finally { setBusy(false); }
  }
  useEffect(() => { void refresh(); }, []);

  async function action(command: string, success: string) {
    try { if (isTauriRuntime()) await invokeCommand(command); onNotify("success", success); await refresh(); }
    catch (error) { onNotify("error", `${success} failed`, error instanceof Error ? error.message : String(error)); }
  }
  const stat = (label: string, value: React.ReactNode, tone?: string) => <div className="diagnostic-stat"><span>{label}</span><strong className={tone}>{value}</strong></div>;
  return (
    <main className="page-shell diagnostics-page">
      <header className="page-header"><div><h1>Diagnostics</h1><p>Inspect local subsystems without exposing paper content.</p></div><button className="button secondary" disabled={busy} onClick={refresh}>{busy && <span className="mini-spinner"/>}Refresh</button></header>
      <section className="diagnostic-actions"><button className="button secondary" onClick={() => action("test_database", "Database check completed")}>Test Database</button><button className="button secondary" onClick={() => action("check_ai_connection", "LM Studio check completed")}>Test LM Studio</button><button className="button secondary" onClick={() => action("open_log_folder", "Log folder opened")}><ExternalIcon/>Open Log Folder</button><button className="button primary" onClick={() => action("copy_diagnostic_summary", "Diagnostic summary copied")}>Copy Diagnostic Summary</button></section>
      <div className="diagnostics-grid">
        <LibraryCachePanel/>
        <section className="diagnostic-card"><h2>Application</h2>{stat("Version", snapshot.application_version)}{stat("Environment", snapshot.environment)}{stat("Database", snapshot.database_status, snapshot.database_status === "OK" ? "good" : undefined)}{stat("Migration", `v${snapshot.migration_version}`)}<div className="diagnostic-path"><span>Database path</span><code>{snapshot.database_path}</code></div><div className="diagnostic-path"><span>Log file</span><code>{snapshot.log_file_location}</code></div></section>
        <section className="diagnostic-card"><h2>Local AI</h2>{stat("Connection", snapshot.lm_studio_status.replaceAll("_", " "), snapshot.lm_studio_status === "connected" ? "good" : "warn")}{stat("Current model", snapshot.current_model || "None")}{stat("Last request", snapshot.last_ai_request_status)}<div className="diagnostic-path"><span>Base URL</span><code>{snapshot.lm_studio_base_url}</code></div></section>
        <section className="diagnostic-card"><h2>Library</h2>{stat("Papers", snapshot.paper_count)}{stat("AI analyzed", snapshot.analyzed_paper_count)}{stat("Last import", snapshot.last_import_status)}</section>
        <section className="diagnostic-card errors-card"><h2>Recent Errors <span>{snapshot.recent_errors.length}</span></h2>{snapshot.recent_errors.length ? snapshot.recent_errors.map((error, index) => <article key={`${error.timestamp}-${index}`}><div><strong>{error.error_code ?? error.operation}</strong><time>{new Date(error.timestamp).toLocaleTimeString()}</time></div><p>{error.message}</p><code>{error.module}{error.request_id ? ` · ${error.request_id}` : ""}</code></article>) : <div className="no-errors">No errors recorded in this session.</div>}</section>
      </div>
    </main>
  );
}
