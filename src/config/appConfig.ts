export const APP_CONFIG = {
  appName: "paper-master",
  version: "0.4.4",
  defaultBaseUrl: "http://127.0.0.1:1234/v1",
  defaultTemperature: 0.6,
  defaultTimeoutMs: 600_000,
  maxAnalysisInputCharacters: 24_000,
  maxOutputTokens: 8_192,
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
