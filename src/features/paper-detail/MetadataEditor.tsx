import { useState } from "react";
import type { Paper } from "../../domain/paper/Paper";
import { normalizeAppError } from "../../types/errors";
import type { EditableMetadata } from "../../domain/paper/Paper";
export type { EditableMetadata } from "../../domain/paper/Paper";
export function MetadataEditor({paper,onSave}:{paper:Paper;onSave(metadata:EditableMetadata):Promise<void>}) {
  const [draft,setDraft]=useState<EditableMetadata>();const [busy,setBusy]=useState(false);const [error,setError]=useState<string>();
  return <section className="metadata-editor"><button className="button secondary" disabled={busy} onClick={()=>{setDraft(draft?undefined:{title:paper.title,authors:[...paper.authors],abstract:paper.abstract,publication_venue:paper.publication_venue,year:paper.year});setError(undefined);}}>{draft?"取消元数据编辑":"编辑题名、作者与摘要"}</button>
    {draft&&<form className="analysis-editor" onSubmit={async event=>{event.preventDefault();setBusy(true);setError(undefined);try{await onSave(draft);setDraft(undefined);}catch(error){setError(normalizeAppError(error).message);}finally{setBusy(false);}}}>
      <label>题名<input required value={draft.title} onChange={e=>setDraft({...draft,title:e.target.value})}/></label>
      <label>作者（每行一位）<textarea aria-label="作者（每行一位）" value={draft.authors.join("\n")} onChange={e=>setDraft({...draft,authors:e.target.value.split("\n")})}/></label>
      <label>刊出主体<input value={draft.publication_venue??""} onChange={e=>setDraft({...draft,publication_venue:e.target.value})}/></label>
      <label>年份<input type="number" min="1900" max="2100" value={draft.year??""} onChange={e=>setDraft({...draft,year:e.target.value?Number(e.target.value):undefined})}/></label>
      <label>摘要<textarea aria-label="摘要" rows={6} value={draft.abstract??""} onChange={e=>setDraft({...draft,abstract:e.target.value})}/></label>
      <button className="button primary" disabled={busy}>保存元数据</button>
    </form>}{error&&<p role="alert" className="analysis-error">{error}</p>}
  </section>;
}
