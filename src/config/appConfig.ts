export const APP_CONFIG = {
  appName: "paper-master",
  version: "0.2.0",
  defaultBaseUrl: "http://127.0.0.1:1234/v1",
  defaultTemperature: 0.15,
  defaultTimeoutMs: 120_000,
  maxAnalysisInputCharacters: 24_000,
  aiStructuredRetryCount: 1,
  logLevel: import.meta.env.DEV ? "DEBUG" : "INFO",
  developmentMode: import.meta.env.DEV,
  featureFlags: {
    semanticSearch: false,
    ragChat: false,
    automaticClustering: false,
    cloudProvider: false,
  },
} as const;
