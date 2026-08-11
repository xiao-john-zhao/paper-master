import { useEffect, useState } from "react";
import type { AIProvider } from "../providers/ai/AIProvider";
import type { AppSettings, SettingsStorage } from "../settings/types";
import { DEFAULT_SETTINGS } from "../settings/InMemorySettingsStorage";
import { CheckIcon, ShieldIcon } from "../components/Icons";
import type { ConnectionResult } from "../domain/ai/types";

interface Props {
  storage: SettingsStorage;
  providers: Record<string, AIProvider>;
  onSaved(settings: AppSettings): void;
  onNotify(tone: "success" | "error" | "info", title: string, message?: string): void;
}

export function SettingsPage({ storage, providers, onSaved, onNotify }: Props) {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [models, setModels] = useState<string[]>([]);
  const [connection, setConnection] = useState<ConnectionResult>();
  const [busy, setBusy] = useState(false);

  useEffect(() => { storage.get().then(setSettings).catch(() => setSettings(DEFAULT_SETTINGS)); }, [storage]);

  const provider = providers[settings.ai_provider];
  async function testConnection() {
    setBusy(true); setConnection(undefined);
    try {
      const discovered = await provider.checkConnection(settings);
      setModels(discovered.models);
      const selectedModel = discovered.models.includes(settings.model) ? settings.model : discovered.models[0] ?? "";
      const nextSettings = { ...settings, model: selectedModel };
      setSettings(nextSettings);
      const result = selectedModel ? await provider.loadModel(nextSettings) : discovered;
      setConnection(result);
    } catch (error) {
      setConnection({ state: "connection_failed", message: error instanceof Error ? error.message : String(error), models: [], loaded_models: [] });
    } finally { setBusy(false); }
  }

  async function save() {
    setBusy(true);
    try {
      const saved = await storage.save(settings);
      onSaved(saved); onNotify("success", "Settings saved", "Provider settings remain on this computer.");
    } catch (error) { onNotify("error", "Settings were not saved", error instanceof Error ? error.message : String(error)); }
    finally { setBusy(false); }
  }

  return (
    <main className="page-shell settings-page">
      <header className="page-header"><div><h1>Settings</h1><p>Configure local analysis and application behavior.</p></div><button className="button primary" disabled={busy} onClick={save}>Save changes</button></header>
      <div className="settings-grid">
        <section className="settings-panel">
          <div className="panel-heading"><div><h2>AI Provider</h2><p>paper-master connects only to a provider running on this computer.</p></div></div>
          <div className="form-grid">
            <label><span>Provider</span><select value={settings.ai_provider} onChange={(event) => { setConnection(undefined); setModels([]); setSettings({ ...settings, ai_provider: event.target.value as AppSettings["ai_provider"] }); }}><option value="lmstudio">LM Studio</option><option value="mock">Mock (development)</option></select></label>
            <label className="full"><span>Base URL</span><input value={settings.base_url} disabled={settings.ai_provider === "mock"} onChange={(event) => setSettings({ ...settings, base_url: event.target.value })}/><small>Only localhost and 127.0.0.1 are allowed in Local Only mode.</small></label>
            <label className="full"><span>Model</span><div className="input-with-action"><select value={settings.model} onChange={(event) => setSettings({ ...settings, model: event.target.value })}><option value="" disabled>请选择本地模型</option>{settings.model && !models.includes(settings.model) && <option value={settings.model}>{settings.model}</option>}{models.map((model) => <option key={model} value={model}>{model}</option>)}</select><button className="button secondary" disabled={busy} onClick={testConnection}>{busy ? <span className="mini-spinner"/> : null}加载并测试</button></div></label>
            {connection && <div className={`connection-result full ${connection.state}`}><span className="connection-icon">{connection.state === "connected" ? <CheckIcon/> : "!"}</span><div><strong>{connection.state === "connected" ? "模型已加载" : connection.state === "model_not_loaded" ? "模型尚未加载" : "连接失败"}</strong><p>{connection.message}</p></div>{connection.latency_ms !== undefined && <span>{connection.latency_ms} ms</span>}</div>}
            <label><span>Temperature</span><div className="range-row"><input type="range" min="0" max="1" step="0.05" value={settings.temperature} onChange={(event) => setSettings({ ...settings, temperature: Number(event.target.value) })}/><output>{settings.temperature.toFixed(2)}</output></div></label>
            <label><span>Request timeout</span><div className="unit-input"><input type="number" min="30" max="600" value={settings.request_timeout_ms / 1000} onChange={(event) => setSettings({ ...settings, request_timeout_ms: Number(event.target.value) * 1000 })}/><span>seconds</span></div></label>
            <label><span>Maximum analysis input</span><div className="unit-input"><input type="number" min="4000" max="100000" step="1000" value={settings.max_analysis_input_characters} onChange={(event) => setSettings({ ...settings, max_analysis_input_characters: Number(event.target.value) })}/><span>characters</span></div></label>
            <label><span>Maximum AI output</span><div className="unit-input"><input type="number" min="512" max="32768" step="256" value={settings.max_output_tokens} onChange={(event) => setSettings({ ...settings, max_output_tokens: Number(event.target.value) })}/><span>tokens</span></div></label>
            <label><span>Log level</span><select value={settings.log_level} onChange={(event) => setSettings({ ...settings, log_level: event.target.value as AppSettings["log_level"] })}>{["TRACE", "DEBUG", "INFO", "WARN", "ERROR"].map((level) => <option key={level}>{level}</option>)}</select></label>
          </div>
        </section>
        <aside className="privacy-panel"><div className="privacy-icon"><ShieldIcon/></div><div><span className="eyebrow">Privacy mode</span><h2>Local Only</h2><p>All paper content and AI requests remain on this computer.</p></div><ul><li><CheckIcon/>PDF files are never copied or uploaded</li><li><CheckIcon/>AI requests are restricted to localhost</li><li><CheckIcon/>No telemetry or analytics</li><li><CheckIcon/>Logs exclude paper text and prompts</li></ul></aside>
      </div>
    </main>
  );
}
