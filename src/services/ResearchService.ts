import { invokeCommand } from "../adapters/tauri/invokeCommand";
import type { ImportSummary } from "../domain/paper/Paper";
import { progressService,taskId } from "./ProgressService";
import { normalizeAppError } from "../types/errors";

export interface ResearchPaper {
  title: string; authors: string[]; year: number | null; venue: string; ieee_url: string;
  doi: string; citation_count: number | null; quality_reason: string; evidence_urls: string[]; open_pdf_url: string;
  relationship: "topic_similar" | "shared_author" | "potential_group";
  relationship_evidence: string; relationship_sources: string[]; matched_authors: string[];
}
export interface SearchIntent {
  summary: string; topic: string; target_authors: string[]; reference_title: string;
  relationship: "any" | "shared_author" | "potential_group";
  year_from: number | null; year_to: number | null; search_queries: string[];
  needs_clarification: boolean; clarification_question: string;
}
export interface ResearchResult { schema_version: string; intent: SearchIntent; papers: ResearchPaper[]; limitations: string }

export class ResearchService {
  async search(request: string, limit: number, requestId: string, referencePaperId?: string) {
    progressService.begin(requestId,"research");
    try { const result=await invokeCommand<ResearchResult>("search_online_papers", { request, referencePaperId, limit, requestId }); progressService.finish(requestId,"completed",result.intent.needs_clarification ? "请补充参考信息" : "检索与关系校验完成"); return result; }
    catch(error) { const appError=normalizeAppError(error); progressService.finish(requestId,appError.code === "AI_CANCELLED" ? "cancelled" : "failed",appError.message); throw error; }
  }
  async download(pdfUrl: string, requestId = taskId("download")) {
    progressService.begin(requestId,"download");
    try { const result=await invokeCommand<ImportSummary>("download_online_paper", { pdfUrl,requestId }); progressService.finish(requestId,result.failed.length ? "failed" : "completed",result.failed.length ? "PDF 已下载，导入失败" : "下载并导入完成"); return result; }
    catch(error) { progressService.finish(requestId,"failed",normalizeAppError(error).message); throw error; }
  }
  openWebVpn(ieeeUrl?: string) { return invokeCommand<void>("open_ieee_webvpn", { ieeeUrl }); }
  openExternal(ieeeUrl?: string) { return invokeCommand<void>("open_ieee_external", { ieeeUrl }); }
  openRelationshipSource(url: string) { return invokeCommand<void>("open_relationship_source", { url }); }
  openCredentials() { return invokeCommand<void>("open_webvpn_credentials"); }
  cancel(requestId: string) { return invokeCommand<boolean>("cancel_ai_request", { requestId }); }
}
