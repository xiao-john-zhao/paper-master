import { useEffect, useState } from "react";
import type { AIProvider } from "../providers/ai/AIProvider";
import type { AppSettings, SettingsStorage } from "../settings/types";
import { DEFAULT_SETTINGS } from "../settings/InMemorySettingsStorage";
import { CheckIcon, ShieldIcon } from "../components/Icons";
import type { ConnectionResult } from "../domain/ai/types";
import { InlineTaskProgress } from "../components/TaskProgress";
import { progressService,taskId } from "../services/ProgressService";

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
  const [progressId,setProgressId] = useState<string>();

  useEffect(() => { storage.get().then(setSettings).catch(() => setSettings(DEFAULT_SETTINGS)); }, [storage]);

  const provider = providers[settings.ai_provider];
  async function testConnection() {
    setBusy(true); setConnection(undefined);
    const id=taskId("settings");setProgressId(id);progressService.begin(id,"settings","检查 AI 服务","正在检查连接与登录状态");
    try {
      const discovered = await provider.checkConnection(settings);
      if (settings.ai_provider === "codex") { setConnection(discovered);progressService.finish(id,discovered.state === "connected"?"completed":"failed",discovered.message); return; }
      setModels(discovered.models);
      const selectedModel = discovered.models.includes(settings.model) ? settings.model : discovered.models[0] ?? "";
      const nextSettings = { ...settings, model: selectedModel };
      setSettings(nextSettings);
      progressService.update({id,kind:"settings",stage:"模型列表已读取，正在加载并检查所选模型"});
      const result = selectedModel ? await provider.loadModel(nextSettings) : discovered;
      setConnection(result);
      progressService.finish(id,result.state === "connected"?"completed":"failed",result.message);
    } catch (error) {
      progressService.finish(id,"failed",error instanceof Error?error.message:String(error));
      setConnection({ state: "connection_failed", message: error instanceof Error ? error.message : String(error), models: [], loaded_models: [] });
    } finally { setBusy(false); }
  }

  async function save() {
    setBusy(true);
    const id=taskId("settings");setProgressId(id);progressService.begin(id,"settings","保存应用设置","正在校验并保存配置");
    try {
      const saved = await storage.save(settings);
      onSaved(saved); onNotify("success", "Settings saved", "Provider settings remain on this computer.");
      progressService.finish(id,"completed","设置已保存");
    } catch (error) { progressService.finish(id,"failed",error instanceof Error?error.message:String(error)); onNotify("error", "Settings were not saved", error instanceof Error ? error.message : String(error)); }
    finally { setBusy(false); }
  }

  return (
    <main className="page-shell settings-page">
      <header className="page-header"><div><h1>Settings</h1><p>选择本地模型或 Codex 阅读总结，并配置联网检索。</p></div><button className="button primary" disabled={busy} onClick={save}>Save changes</button></header>
      <div className="settings-grid">
        {progressId&&<div className="settings-operation-progress"><InlineTaskProgress id={progressId}/></div>}
        <section className="settings-panel">
          <div className="panel-heading"><div><h2>AI Provider</h2><p>LM Studio 在本机运行；Codex 通过已登录的 CLI 调用在线服务。</p></div></div>
          <div className="form-grid">
            <label className="full auto-analysis-toggle"><span><input type="checkbox" checked={settings.auto_analyze_on_import} onChange={(event) => setSettings({ ...settings, auto_analyze_on_import: event.target.checked })}/>首次导入后自动分析</span><small>导入成功后进入本地分析队列，依次生成问题、场景、方法、创新、贡献和概括。关闭应用后会在下次启动时继续。</small></label>
            <label><span>Provider</span><select value={settings.ai_provider} onChange={(event) => { setConnection(undefined); setModels([]); setSettings({ ...settings, ai_provider: event.target.value as AppSettings["ai_provider"] }); }}><option value="lmstudio">LM Studio</option><option value="codex">Codex CLI（在线阅读与总结）</option><option value="mock">Mock (development)</option></select></label>
            <label className="full auto-analysis-toggle"><span><input type="checkbox" checked={settings.privacy_mode === "codex_online"} onChange={(event) => setSettings({ ...settings, privacy_mode: event.target.checked ? "codex_online" : "local_only" })}/>启用联网功能（同一应用内切换）</span><small>检索会发送需求文本及所选参考论文的题名/作者信息；选择 Codex 为 AI 服务后，手动分析、自动队列及比较会把有界论文文本发送至 Codex 在线服务。学校凭据不会发送给 Codex。</small></label>
            {settings.ai_provider !== "codex" && <label className="full"><span>Base URL</span><input value={settings.base_url} disabled={settings.ai_provider === "mock"} onChange={(event) => setSettings({ ...settings, base_url: event.target.value })}/><small>LM Studio 只允许 localhost 和 127.0.0.1。</small></label>}
            {settings.ai_provider === "codex" ? <div className="full"><p>使用本机 Codex CLI 的默认模型。需安装 Codex 并完成 codex login。温度、推理强度、加载上下文和输出 token 设置仅用于本地模型。</p><button className="button secondary" disabled={busy} onClick={testConnection}>检查 Codex 登录状态</button></div> : <label className="full"><span>Model</span><div className="input-with-action"><select value={settings.model} onChange={(event) => setSettings({ ...settings, model: event.target.value })}><option value="" disabled>请选择本地模型</option>{settings.model && !models.includes(settings.model) && <option value={settings.model}>{settings.model}</option>}{models.map((model) => <option key={model} value={model}>{model}</option>)}</select><button className="button secondary" disabled={busy} onClick={testConnection}>{busy ? <span className="mini-spinner"/> : null}加载并测试</button></div></label>}
            {connection && <div className={`connection-result full ${connection.state}`}><span className="connection-icon">{connection.state === "connected" ? <CheckIcon/> : "!"}</span><div><strong>{connection.state === "connected" ? "模型已加载" : connection.state === "model_not_loaded" ? "模型尚未加载" : "连接失败"}</strong><p>{connection.message}</p></div>{connection.latency_ms !== undefined && <span>{connection.latency_ms} ms</span>}</div>}
            <label><span>Temperature</span><div className="range-row"><input type="range" min="0" max="1" step="0.05" value={settings.temperature} onChange={(event) => setSettings({ ...settings, temperature: Number(event.target.value) })}/><output>{settings.temperature.toFixed(2)}</output></div></label>
            <label><span>推理强度</span><select value={settings.reasoning_effort} onChange={(event) => setSettings({ ...settings, reasoning_effort: event.target.value as AppSettings["reasoning_effort"] })}><option value="none">关闭额外推理（推荐批量分析）</option><option value="low">低</option><option value="medium">中</option><option value="high">高</option><option value="xhigh">极高</option></select><small>较高强度需要更长时间和输出预算；仅适用于支持该参数的模型。</small></label>
            <label><span>Request timeout</span><div className="unit-input"><input type="number" min="30" max="600" value={settings.request_timeout_ms / 1000} onChange={(event) => setSettings({ ...settings, request_timeout_ms: Number(event.target.value) * 1000 })}/><span>seconds</span></div></label>
            <label><span>Maximum analysis input</span><div className="unit-input"><input type="number" min="4000" max="100000" step="1000" value={settings.max_analysis_input_characters} onChange={(event) => setSettings({ ...settings, max_analysis_input_characters: Number(event.target.value) })}/><span>characters</span></div></label>
            <label><span>模型加载上下文</span><div className="unit-input"><input type="number" min="4096" max="131072" step="1024" value={settings.model_context_length} onChange={(event) => setSettings({ ...settings, model_context_length: Number(event.target.value) })}/><span>tokens</span></div><small>用于下次模型加载；已加载模型需在 LM Studio 卸载后重新加载。多篇对比共用最大分析输入预算。</small></label>
            <label><span>Maximum AI output</span><div className="unit-input"><input type="number" min="512" max="32768" step="256" value={settings.max_output_tokens} onChange={(event) => setSettings({ ...settings, max_output_tokens: Number(event.target.value) })}/><span>tokens</span></div></label>
            <label><span>Log level</span><select value={settings.log_level} onChange={(event) => setSettings({ ...settings, log_level: event.target.value as AppSettings["log_level"] })}>{["TRACE", "DEBUG", "INFO", "WARN", "ERROR"].map((level) => <option key={level}>{level}</option>)}</select></label>
          </div>
        </section>
        <aside className="privacy-panel"><div className="privacy-icon"><ShieldIcon/></div><div><span className="eyebrow">Privacy mode</span><h2>{settings.privacy_mode === "codex_online" ? "Codex 联网已授权" : "本地分析"}</h2><p>{settings.privacy_mode === "codex_online" ? "联网检索与 Codex 分析使用在线服务；关闭开关并保存后，取消正在执行的任务并关闭学校浏览器窗口。" : "联网功能已关闭。Codex 选择会保留，分析自动使用本机 LM Studio；已保存的所有模型结果仍可查看和编辑。"}</p></div><ul><li><CheckIcon/>导入的原 PDF 保留原位置</li><li><CheckIcon/>学校凭据仅保存在独立私有 TXT</li><li><CheckIcon/>无应用遥测或分析跟踪</li><li><CheckIcon/>诊断日志不记录论文、提示词和密码</li></ul></aside>
      </div>
    </main>
  );
}
