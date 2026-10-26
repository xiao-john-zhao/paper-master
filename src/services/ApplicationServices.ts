import { isTauriRuntime } from "../adapters/tauri/invokeCommand";
import { InMemoryPaperRepository } from "../repositories/InMemoryPaperRepository";
import { TauriPaperRepository } from "../repositories/TauriPaperRepository";
import { LMStudioProvider } from "../providers/ai/LMStudioProvider";
import { MockAIProvider } from "../providers/ai/MockAIProvider";
import { KeywordSearchProvider } from "../providers/search/KeywordSearchProvider";
import { DEFAULT_SETTINGS, InMemorySettingsStorage } from "../settings/InMemorySettingsStorage";
import { TauriSettingsStorage } from "../settings/TauriSettingsStorage";
import { AnalysisService } from "./AnalysisService";
import { PaperService } from "./PaperService";
import { ComparisonService } from "./ComparisonService";
import type { AIProvider } from "../providers/ai/AIProvider";
import { ResearchService } from "./ResearchService";
import { RelationshipGraphService } from "./RelationshipGraphService";

import { AnalysisVersionsService } from "./AnalysisVersionsService";

export function createApplicationServices() {
  const tauri = isTauriRuntime();
  const repository = tauri ? new TauriPaperRepository() : new InMemoryPaperRepository();
  const settingsStorage = tauri ? new TauriSettingsStorage() : new InMemorySettingsStorage({ ...DEFAULT_SETTINGS, ai_provider: "mock" });
  const providers: Record<string, AIProvider> = { openai: new LMStudioProvider("openai"), lmstudio: new LMStudioProvider(), codex: new LMStudioProvider("codex"), mock: tauri ? new LMStudioProvider("mock") : new MockAIProvider() };
  const searchProvider = tauri ? new KeywordSearchProvider() : undefined;
  const analysisService = new AnalysisService(tauri ? providers.lmstudio : providers.mock, repository);
  const paperService = new PaperService(repository,searchProvider);

  return {
    tauri,
    researchService: new ResearchService(),
    repository,
    settingsStorage,
    providers,
    paperService,
    graphService: new RelationshipGraphService(paperService),
    analysisService,
    analysisVersionsService: new AnalysisVersionsService(repository),
    comparisonService: new ComparisonService(tauri ? providers.lmstudio : providers.mock, analysisService, repository),
  };
}

export type ApplicationServices = ReturnType<typeof createApplicationServices>;
