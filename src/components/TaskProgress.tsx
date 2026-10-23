import { useEffect, useState, useSyncExternalStore } from "react";
import { progressService, type TaskProgress as Task } from "../services/ProgressService";

export function useTasks() { return useSyncExternalStore(progressService.subscribe, progressService.snapshot); }
function formatSize(value: number) { return value >= 1048576 ? `${(value / 1048576).toFixed(1)} MB` : `${Math.round(value / 1024)} KB`; }
export function TaskProgressBar({ task, compact = false }: { task: Task; compact?: boolean }) {
  const [now,setNow] = useState(Date.now());
  useEffect(() => { if (task.status !== "running") return; const timer = window.setInterval(() => setNow(Date.now()),1000); return () => window.clearInterval(timer); },[task.status]);
  const elapsed = Math.max(0,Math.floor(((task.status === "running" ? now : task.updatedAt) - task.startedAt) / 1000));
  const count = task.total !== undefined && task.completed !== undefined;
  const label = count ? task.unit === "bytes" ? `${formatSize(task.completed!)} / ${formatSize(task.total!)}` : task.unit === "steps" ? `${task.completed}/${task.total} 阶段` : `${task.completed}/${task.total} 文件` : task.status === "running" ? "处理中" : "";
  return <div className={`task-progress ${compact ? "compact" : ""} ${task.status}`}>
    <div className="task-progress-heading"><strong>{task.title}</strong><span>{label}{elapsed ? ` · ${elapsed >= 60 ? `${Math.floor(elapsed/60)}分${elapsed%60}秒` : `${elapsed}秒`}` : ""}</span></div>
    <progress aria-label={`${task.title}进度`} max={count ? task.total : 1} value={task.status === "completed" ? (task.total ?? 1) : count ? task.completed : task.status !== "running" ? 0 : undefined}/>
    <div className="task-progress-stage" role={task.status === "failed" ? "alert" : "status"}>{task.stage}</div>
  </div>;
}
export function InlineTaskProgress({ id,fallback }: { id?: string;fallback?:string }) {
  const tasks = useTasks(); const task = tasks.find(value => value.id === id);
  return task ? <TaskProgressBar task={task}/> : fallback ? <div className="task-progress"><div className="task-progress-stage" role="status">{fallback}</div><progress aria-label="任务正在处理"/></div> : null;
}
export function TaskCenter() {
  const tasks = useTasks(); const [expanded,setExpanded] = useState(false);
  const active = tasks.filter(task => task.status === "running");
  const failed = tasks.filter(task => task.status === "failed");
  if (!tasks.length) return null;
  return <aside className={`task-center ${expanded ? "expanded" : ""}`} aria-label="任务进度中心">
    <button className="task-center-toggle" aria-expanded={expanded} onClick={() => setExpanded(!expanded)}><span className={active.length ? "mini-spinner" : failed.length ? "task-failure" : "task-check"}>{active.length ? "" : failed.length ? "!" : "✓"}</span><strong>{active.length ? `${active.length} 项任务进行中` : failed.length ? `${failed.length} 项任务失败` : "任务已结束"}</strong><span>{expanded ? "收起" : "查看进度"}</span></button>
    {expanded && <div className="task-center-content">{[...active,...tasks.filter(task => task.status !== "running")].map(task => <TaskProgressBar key={task.id} task={task} compact/>)}<button className="text-button" onClick={() => progressService.clearFinished()}>清除已结束任务</button></div>}
  </aside>;
}
