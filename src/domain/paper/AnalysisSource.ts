import type { AnalysisVersion } from "../../services/AnalysisVersionsService";

export function analysisSource(version: AnalysisVersion) {
  if (version.provider === "extractive") return `原文摘录（非模型）${version.edited ? " · 人工修订" : ""}`;
  if (version.provider === "legacy") return "历史分析 · 原模型未记录";
  if (version.provider === "mock") return `演示模型 · ${version.model || "Mock"}`;
  const provider = version.provider === "lmstudio" ? "LM Studio" : version.provider === "codex" ? "Codex" : version.provider === "openai" ? "外部 API" : version.provider;
  const unknown = !version.model || ["auto", "Codex CLI default"].includes(version.model);
  return `${provider} · ${unknown ? "具体模型未记录" : version.model}`;
}

export function analysisTime(stamp: string) {
  const date = new Date(stamp);
  return Number.isNaN(date.valueOf()) ? "时间未记录" : new Intl.DateTimeFormat("zh-CN", {year:"numeric",month:"2-digit",day:"2-digit",hour:"2-digit",minute:"2-digit",second:"2-digit",hour12:false}).format(date);
}

export function analysisVersionLabel(version: AnalysisVersion) {
  return `${analysisSource(version)} | ${analysisTimeLabel(version)}：${analysisTime(version.created_at)}${version.edited ? ` | 修订 ${version.revision}` : ""}`;
}

export function analysisTimeLabel(version: AnalysisVersion) {
  return version.provider === "legacy" ? "历史记录时间（总结时间未记录）" : version.provider === "extractive" ? "摘录时间" : "总结时间";
}
