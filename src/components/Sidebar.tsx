import type { LibraryFilter } from "../domain/paper/Paper";
import { ClockIcon, DiagnosticsIcon, LibraryIcon, SettingsIcon, SparkIcon,GraphIcon } from "./Icons";

export type AppView = "library" | "settings" | "diagnostics" | "research" | "graph";

interface Props {
  view: AppView;
  filter: LibraryFilter;
  tags: Array<{ name: string; count: number }>;
  activeTags: string[];
  onlineMode?: boolean;
  onView(view: AppView): void;
  onFilter(filter: LibraryFilter): void;
  onTag(tag: string): void;
  onClearTags(): void;
}

export function Sidebar({ view, filter, tags, activeTags, onlineMode, onView, onFilter, onTag, onClearTags }: Props) {
  const nav = (value: LibraryFilter, icon: React.ReactNode, label: string) => (
    <button className={`side-item ${view === "library" && filter === value ? "active" : ""}`} onClick={() => { onView("library"); onFilter(value); }}>
      {icon}<span>{label}</span>
    </button>
  );

  return (
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark">PM</span><span>paper-master</span></div>
      <div className="side-section">
        <div className="side-heading">文献工作台</div>
        {nav("all", <LibraryIcon />, "全部论文")}
        {nav("recent", <ClockIcon />, "最近添加")}
        {nav("analyzed", <SparkIcon />, "已完成分析")}
        <button className={`side-item ${view === "research" ? "active" : ""}`} onClick={() => onView("research")}><SparkIcon/><span>联网检索论文</span></button>
        <button className={`side-item ${view === "graph" ? "active" : ""}`} onClick={() => onView("graph")}><GraphIcon/><span>论文关系图</span></button>
      </div>
      <div className="side-section tags-section">
        <div className="side-heading side-heading-row"><span>关键词 · 多选交集</span><span>{tags.length}</span></div>
        {!!activeTags.length && <button className="text-button" onClick={onClearTags}>清除所选（{activeTags.length}）</button>}
        <div className="tag-nav">
          {tags.map((tag) => (
            <label key={tag.name} className={`tag-nav-item ${activeTags.includes(tag.name) ? "active" : ""}`}>
              <input type="checkbox" checked={activeTags.includes(tag.name)} onChange={() => { onView("library"); onTag(tag.name); }}/><span className="tag-name">{tag.name}</span><span className="tag-count">{tag.count}</span>
            </label>
          ))}
          {!tags.length && <span className="side-muted">Tags appear after import or analysis.</span>}
        </div>
      </div>
      <div className="sidebar-footer">
        <button className={`side-item ${view === "settings" ? "active" : ""}`} onClick={() => onView("settings")}><SettingsIcon/><span>Settings</span></button>
        <button className={`side-item ${view === "diagnostics" ? "active" : ""}`} onClick={() => onView("diagnostics")}><DiagnosticsIcon/><span>Diagnostics</span></button>
        <div className="local-badge"><span className="status-dot online"/>{onlineMode ? "Codex 联网已授权" : "本地分析"}</div>
      </div>
    </aside>
  );
}

