import { isTauriRuntime } from "../adapters/tauri/invokeCommand";
import { InMemoryPaperRepository } from "../repositories/InMemoryPaperRepository";
import { TauriPaperRepository } from "../repositories/TauriPaperRepository";
import { LMStudioProvider } from "../providers/ai/LMStudioProvider";
import { MockAIProvider } from "../providers/ai/MockAIProvider";
import { KeywordSearchProvider } from "../providers/search/KeywordSearchProvider";
import { InMemorySettingsStorage } from "../settings/InMemorySettingsStorage";
import { TauriSettingsStorage } from "../settings/TauriSettingsStorage";
import { AnalysisService } from "./AnalysisService";
import { PaperService } from "./PaperService";
import { ComparisonService } from "./ComparisonService";
import type { AIProvider } from "../providers/ai/AIProvider";

export function createApplicationServices() {
  const tauri = isTauriRuntime();
  const repository = tauri ? new TauriPaperRepository() : new InMemoryPaperRepository();
  const settingsStorage = tauri ? new TauriSettingsStorage() : new InMemorySettingsStorage();
  const providers: Record<string, AIProvider> = { lmstudio: new LMStudioProvider(), mock: new MockAIProvider() };
  const searchProvider = tauri ? new KeywordSearchProvider() : undefined;

  return {
    tauri,
    repository,
    settingsStorage,
    providers,
    paperService: new PaperService(repository, searchProvider),
    analysisService: new AnalysisService(tauri ? providers.lmstudio : providers.mock, repository),
    comparisonService: new ComparisonService(tauri ? providers.lmstudio : providers.mock),
  };
}

export type ApplicationServices = ReturnType<typeof createApplicationServices>;
