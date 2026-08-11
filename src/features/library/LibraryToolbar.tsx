import { CompareIcon, FolderIcon, PlusIcon, SearchIcon } from "../../components/Icons";

interface Props {
  search: string;
  compareCount: number;
  loading: boolean;
  onSearch(value: string): void;
  onImportPdf(): void;
  onImportFolder(): void;
  onCompare(): void;
}

export function LibraryToolbar({ search, compareCount, loading, onSearch, onImportPdf, onImportFolder, onCompare }: Props) {
  return (
    <header className="library-toolbar">
      <label className="search-box"><SearchIcon/><input value={search} onChange={(event) => onSearch(event.target.value)} placeholder="Search title, authors, abstract, tags…" aria-label="Search papers"/>{loading && <span className="mini-spinner"/>}</label>
      <div className="toolbar-actions">
        <button className="button secondary compact" onClick={onImportFolder}><FolderIcon/>Folder</button>
        <button className="button primary compact" onClick={onImportPdf}><PlusIcon/>Import PDF</button>
        <button className="button secondary compact" disabled={compareCount !== 2} onClick={onCompare}><CompareIcon/>Compare{compareCount ? ` (${compareCount}/2)` : ""}</button>
      </div>
    </header>
  );
}

