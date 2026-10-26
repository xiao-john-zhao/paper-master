import type { Paper, PaperAnalysisResult } from "../domain/paper/Paper";
import type { PaperRepository } from "../repositories/PaperRepository";
import { invokeCommand, isTauriRuntime } from "../adapters/tauri/invokeCommand";

export interface AnalysisVersion {
  id:string; paper_id:string; provider:string; model:string; created_at:string; updated_at:string;
  profile?:{attempts:number;input_budget:number;input_characters:number;output_tokens:number;reasoning_effort:string;requested_context_length:number;reading?:{
    strategy:string;source_characters:number;covered_characters:number;consolidation_rounds:number;
    baseline?:{summary:string};audit?:{verdict:string;reason:string;quotes:string[]};
    parts:Array<{start:number;end:number;input_budget:number;reasoning_effort:string;note:{summary:string;claims:Array<{category:string;finding:string;quote:string}>}}>;
  }};
  revision:number; edited:boolean; active:boolean; result:PaperAnalysisResult;
}
export class AnalysisVersionsService {
  readonly desktop = isTauriRuntime();
  private readonly originals = new Map<string,PaperAnalysisResult>();
  private readonly cache = new Map<string,AnalysisVersion[]>();
  constructor(private readonly papers:PaperRepository) {}
  async list(paper:Paper):Promise<AnalysisVersion[]> {
    if (isTauriRuntime()) return invokeCommand("list_analysis_versions",{paperId:paper.id});
    paper=await this.papers.findById(paper.id)??paper;
    if (!this.cache.has(paper.id) || (!this.cache.get(paper.id)?.length && paper.analysis)) this.cache.set(paper.id,paper.analysis ? [{id:`preview-${paper.id}`,paper_id:paper.id,provider:"mock",model:"界面演示",created_at:paper.created_at,updated_at:paper.updated_at,revision:0,edited:false,active:true,result:structuredClone(paper.analysis)}] : []);
    const versions=this.cache.get(paper.id)!;
    if(paper.analysis && versions.length && JSON.stringify(versions.find(v=>v.active)?.result)!==JSON.stringify(paper.analysis)) {
      for(const version of versions)version.active=false;
      versions.unshift({id:`preview-${paper.id}-${paper.updated_at}`,paper_id:paper.id,provider:"mock",model:"界面演示",created_at:paper.updated_at,updated_at:paper.updated_at,revision:0,edited:false,active:true,result:structuredClone(paper.analysis)});
    }
    return structuredClone(versions);
  }
  async select(version:AnalysisVersion) {
    if (isTauriRuntime()) return invokeCommand<AnalysisVersion>("select_analysis_version",{paperId:version.paper_id,id:version.id});
    for (const item of this.cache.get(version.paper_id) ?? []) item.active=item.id===version.id;
    await this.project(version); return {...version,active:true};
  }
  async edit(version:AnalysisVersion,result:PaperAnalysisResult) {
    if (isTauriRuntime()) return invokeCommand<AnalysisVersion>("edit_analysis_version",{paperId:version.paper_id,id:version.id,revision:version.revision,result});
    const item=this.cache.get(version.paper_id)?.find(item=>item.id===version.id);
    if (!item || item.revision!==version.revision) throw new Error("结果已修改，请刷新后重试。");
    if (!this.originals.has(item.id)) this.originals.set(item.id,structuredClone(item.result));
    Object.assign(item,{result:structuredClone(result),edited:true,revision:item.revision+1,updated_at:new Date().toISOString()});
    if (item.active) await this.project(item); return structuredClone(item);
  }
  restore(version:AnalysisVersion) { return isTauriRuntime() ? invokeCommand<AnalysisVersion>("restore_analysis_version",{paperId:version.paper_id,id:version.id,revision:version.revision}) : this.edit(version,structuredClone(this.originals.get(version.id)??version.result)); }
  extract(paper:Paper) {
    return invokeCommand<AnalysisVersion>("extract_paper_locally",{paperId:paper.id});
  }
  private async project(version:AnalysisVersion) {
    const paper=await this.papers.findById(version.paper_id);
    if (!paper) throw new Error("论文不存在。");
    await this.papers.update({...paper,analysis:version.result,tags:version.result.tags,updated_at:new Date().toISOString()});
  }
}
