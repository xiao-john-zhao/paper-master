import type { LibraryFilter } from "../domain/paper/Paper";
import { ClockIcon, DiagnosticsIcon, LibraryIcon, SettingsIcon, SparkIcon } from "./Icons";

export type AppView = "library" | "settings" | "diagnostics";

interface Props {
  view: AppView;
  filter: LibraryFilter;
  tags: Array<{ name: string; count: number }>;
  activeTag?: string;
  developmentMode: boolean;
  onView(view: AppView): void;
  onFilter(filter: LibraryFilter): void;
  onTag(tag?: string): void;
}

export function Sidebar({ view, filter, tags, activeTag, developmentMode, onView, onFilter, onTag }: Props) {
  const nav = (value: LibraryFilter, icon: React.ReactNode, label: string) => (
    <button className={`side-item ${view === "library" && filter === value && !activeTag ? "active" : ""}`} onClick={() => { onView("library"); onTag(undefined); onFilter(value); }}>
      {icon}<span>{label}</span>
    </button>
  );

  return (
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark">PM</span><span>paper-master</span></div>
      <div className="side-section">
        <div className="side-heading">Library</div>
        {nav("all", <LibraryIcon />, "All Papers")}
        {nav("recent", <ClockIcon />, "Recently Added")}
        {nav("analyzed", <SparkIcon />, "AI Analyzed")}
      </div>
      <div className="side-section tags-section">
        <div className="side-heading side-heading-row"><span>Tags</span><span>{tags.length}</span></div>
        <div className="tag-nav">
          {tags.slice(0, 12).map((tag) => (
            <button key={tag.name} className={`tag-nav-item ${activeTag === tag.name ? "active" : ""}`} onClick={() => { onView("library"); onTag(activeTag === tag.name ? undefined : tag.name); }}>
              <span className="tag-dot"/><span className="tag-name">{tag.name}</span><span className="tag-count">{tag.count}</span>
            </button>
          ))}
          {!tags.length && <span className="side-muted">Tags appear after import or analysis.</span>}
        </div>
      </div>
      <div className="sidebar-footer">
        <button className={`side-item ${view === "settings" ? "active" : ""}`} onClick={() => onView("settings")}><SettingsIcon/><span>Settings</span></button>
        {developmentMode && <button className={`side-item ${view === "diagnostics" ? "active" : ""}`} onClick={() => onView("diagnostics")}><DiagnosticsIcon/><span>Diagnostics</span></button>}
        <div className="local-badge"><span className="status-dot online"/>Local only</div>
      </div>
    </aside>
  );
}

