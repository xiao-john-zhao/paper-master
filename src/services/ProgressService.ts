export type TaskStatus = "running" | "completed" | "failed" | "cancelled";
export interface TaskUpdate {
  id: string; kind: string; stage: string; status?: TaskStatus;
  completed?: number; total?: number; unit?: "files" | "bytes" | "steps";
}
export interface TaskProgress extends TaskUpdate { title: string; status: TaskStatus; startedAt: number; updatedAt: number }
const titles: Record<string,string> = { import:"导入论文", analysis:"论文阅读与分析", compare:"论文深度对比", research:"理解需求与检索", download:"下载论文", graph:"构建关系图", export:"导出关系图片", settings:"连接与设置" };

export class ProgressService {
  private tasks: TaskProgress[] = [];
  private listeners = new Set<() => void>();
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
  snapshot = () => this.tasks;
  begin(id: string, kind: string, title = titles[kind] ?? "正在处理", stage = "正在准备") {
    this.tasks = this.tasks.filter(task => task.id !== id);
    this.update({ id, kind, stage, status:"running" }, title);
  }
  update(update: TaskUpdate, title?: string) {
    const previous = this.tasks.find(task => task.id === update.id);
    // Late transport events must not turn a cancelled or completed task back into a running task.
    if (previous && previous.status !== "running" && (update.status ?? "running") === "running") return;
    const now = Date.now();
    const total = update.total && Number.isFinite(update.total) && update.total > 0 ? update.total : undefined;
    const completed = total && update.completed !== undefined && Number.isFinite(update.completed) ? Math.max(0, Math.min(total, update.completed)) : undefined;
    const task: TaskProgress = { ...update, completed, total, title:title ?? previous?.title ?? titles[update.kind] ?? "后台任务", status:update.status ?? "running", startedAt:previous?.startedAt ?? now, updatedAt:now };
    const others = this.tasks.filter(value => value.id !== task.id);
    this.tasks = [task, ...others].filter((value,index) => value.status === "running" || index < 18);
    this.listeners.forEach(listener => listener());
  }
  finish(id: string, status: Exclude<TaskStatus,"running"> = "completed", stage = "已完成") {
    const task = this.tasks.find(value => value.id === id);
    if (!task || (task.status !== "running" && status === "completed")) return;
    this.update({ ...task, status, stage, completed:status === "completed" ? task.total : task.completed });
  }
  clearFinished() { this.tasks = this.tasks.filter(task => task.status === "running"); this.listeners.forEach(listener => listener()); }
}
export const progressService = new ProgressService();
export const taskId = (kind: string) => `${kind}-${crypto.randomUUID()}`;
