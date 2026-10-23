import { buildRelationshipGraph,layoutGraph,graphSvg,type GraphOptions,type RelationshipGraph } from "../domain/paper/RelationshipGraph";
import type { PaperService } from "./PaperService";
import { invokeCommand,isTauriRuntime } from "../adapters/tauri/invokeCommand";
import { progressService,taskId } from "./ProgressService";
import { normalizeAppError } from "../types/errors";

const paint=()=>new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
export class RelationshipGraphService {
  constructor(private readonly papers:PaperService) {}
  async build(options:GraphOptions,id=taskId("graph")) {
    progressService.begin(id,"graph");
    try {
      progressService.update({id,kind:"graph",stage:"读取完整本地论文库",completed:0,total:3,unit:"steps"});
      const papers=await this.papers.list(); await paint();
      const teamLinks=isTauriRuntime() && options.mode!=="keywords" && options.includeTeams!==false ? await this.papers.teamLinks() : [];
      const graph=buildRelationshipGraph(papers,{...options,teamLinks});
      progressService.update({id,kind:"graph",stage:"共同关键词和共同作者已统计，正在布局",completed:1,total:3,unit:"steps"}); await paint();
      const laidOut=layoutGraph(graph);
      progressService.update({id,kind:"graph",stage:"节点与连线布局完成",completed:3,total:3,unit:"steps"});
      progressService.finish(id,"completed","本地关系图已生成");
      return {graph:laidOut,papers};
    } catch(error) { progressService.finish(id,"failed",normalizeAppError(error).message); throw error; }
  }
  async export(graph:RelationshipGraph,format:"svg"|"png",id=taskId("export")) {
    progressService.begin(id,"export");
    try {
      progressService.update({id,kind:"export",stage:"准备完整图形与标签图例",completed:0,total:3,unit:"steps"});await paint();
      const svg=graphSvg(graph); let data:Uint8Array;
      if(format==="svg") data=new TextEncoder().encode(svg);
      else {
        progressService.update({id,kind:"export",stage:"正在渲染高清 PNG",completed:1,total:3,unit:"steps"});
        const url=URL.createObjectURL(new Blob([svg],{type:"image/svg+xml;charset=utf-8"}));
        try {
          const image=new Image(); image.src=url; await image.decode();
          const canvas=document.createElement("canvas");canvas.width=image.naturalWidth;canvas.height=image.naturalHeight;
          if(canvas.width*canvas.height>16_000_000) throw new Error("图像过大，请降低显示节点数后导出。");
          const context=canvas.getContext("2d");if(!context)throw new Error("当前环境无法绘制 PNG，请选择 SVG 导出。");context.drawImage(image,0,0);
          const png=await new Promise<Blob>((resolve,reject)=>canvas.toBlob(blob=>blob?resolve(blob):reject(new Error("PNG 编码失败。")),"image/png"));
          data=new Uint8Array(await png.arrayBuffer());
        } finally { URL.revokeObjectURL(url); }
      }
      progressService.update({id,kind:"export",stage:"图片已生成，请选择保存位置",completed:2,total:3,unit:"steps"});
      if(isTauriRuntime()) {
        const path=await invokeCommand<string|null>("save_graph_image",{format,data:Array.from(data)});
        progressService.finish(id,path ? "completed" : "cancelled",path ? "关系图片已保存" : "已取消保存");return path;
      }
      const copy=new Uint8Array(data);const url=URL.createObjectURL(new Blob([copy],{type:format==="svg"?"image/svg+xml":"image/png"}));
      const link=document.createElement("a");link.href=url;link.download=`paper-master-${graph.mode}.${format}`;link.click();window.setTimeout(()=>URL.revokeObjectURL(url),1000);
      progressService.finish(id,"completed","关系图片已交给浏览器下载");return link.download;
    } catch(error) {progressService.finish(id,"failed",normalizeAppError(error).message);throw error;}
  }
}
