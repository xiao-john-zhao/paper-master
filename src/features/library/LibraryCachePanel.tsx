import { useEffect, useState } from "react";
import { isTauriRuntime } from "../../adapters/tauri/invokeCommand";
import { LibraryCacheService, type LibraryCacheStatus } from "../../services/LibraryCacheService";
import { normalizeAppError } from "../../types/errors";

const service = new LibraryCacheService();

export function LibraryCachePanel() {
  const [status, setStatus] = useState<LibraryCacheStatus>();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const desktop = isTauriRuntime();
  useEffect(() => { if (desktop) void service.status().then(setStatus).catch(error => setMessage(normalizeAppError(error).message)); }, [desktop]);
  async function action(kind: "sync" | "recover" | "openFolder" | "relocate") {
    setBusy(true); setMessage("");
    try {
      if (kind === "openFolder") await service.openFolder();
      else {
        const result = await service[kind]();
        if (!result) return;
        setStatus(result);
        setMessage(kind === "relocate" ? "缓存已复制、校验并切换至新位置，原文件夹保留备份。" : kind === "recover" ? `已补回 ${result.restored} 条缺失的论文或结果记录。` : "论文缓存已同步。");
      }
    } catch (error) { setMessage(normalizeAppError(error).message); }
    finally { setBusy(false); }
  }
  return <section className="diagnostic-card">
    <h2>论文文件缓存</h2>
    <p>分析、结果切换和人工编辑后自动保存，启动时自动补回缺失记录。缓存目录不随应用版本或启动位置改变。</p>
    {desktop ? <>
      <div className="diagnostic-path"><span>缓存文件夹</span><code>{status?.directory ?? "正在读取…"}</code></div>
      {status && <p>格式 v{status.schema_version} · 待同步 {status.pending} 条 · 本次恢复 {status.restored} 条</p>}
      {status?.last_error && <p role="alert">{status.last_error}</p>}
      {!!status?.unreadable && <p role="alert">{status.unreadable} 份缓存无法直接读取，已尝试上一份备份；原文件保留，请核对。</p>}
      <div className="diagnostic-actions">
        <button className="button secondary" disabled={busy} onClick={() => void action("openFolder")}>打开缓存文件夹</button>
        <button className="button secondary" disabled={busy} onClick={() => void action("relocate")}>更改位置 / 读取搬运的缓存</button>
        <button className="button secondary" disabled={busy} onClick={() => void action("sync")}>立即同步缓存</button>
        <button className="button secondary" disabled={busy} onClick={() => void action("recover")}>从缓存补回缺失记录</button>
      </div>
      <p>选择新文件夹后会复制并校验缓存再切换位置，原目录保留备份。手动搬运后可选择搬运后的文件夹继续读取；移动硬盘盘符变化时可重新选择。</p>
      <p>修改论文元数据或整理文字，请使用论文详情中的编辑入口。恢复保留数据库中已有的编辑。</p>
    </> : <p>文件缓存用于桌面应用；浏览器预览使用演示数据。</p>}
    {message && <p role="status">{message}</p>}
  </section>;
}
