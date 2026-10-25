import { CompareIcon, FolderIcon, PlusIcon, SearchIcon } from "../../components/Icons";

interface Props {
  search: string;
  compareCount: number;
  loading: boolean;
  onSearch(value: string): void;
  onImportPdf(): void;
  onImportFolder(): void;
  onCompare(): void;
  onFillMissing?(): void;
}

export function LibraryToolbar({ search, compareCount, loading, onSearch, onImportPdf, onImportFolder, onCompare, onFillMissing }: Props) {
  return (
    <header className="library-toolbar">
      <label className="search-box"><SearchIcon/><input value={search} onChange={(event) => onSearch(event.target.value)} placeholder="Search title, authors, abstract, tags…" aria-label="Search papers"/>{loading && <span className="mini-spinner"/>}</label>
      <div className="toolbar-actions">
        <button className="button secondary compact" onClick={onImportFolder}><FolderIcon/>Folder</button>
        <button className="button primary compact" onClick={onImportPdf}><PlusIcon/>Import PDF</button>
        <button className="button secondary compact" disabled={compareCount < 2} onClick={onCompare}><CompareIcon/>Compare{compareCount ? ` (${compareCount})` : ""}</button>
        {onFillMissing && <button className="button secondary compact" onClick={onFillMissing} title="使用 Settings 中保存的真实模型，补齐当前筛选内的未分析论文；已有结果和人工修改保留">补齐待分析</button>}
      </div>
    </header>
  );
}
