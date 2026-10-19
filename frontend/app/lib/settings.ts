const SETTINGS_KEY = "sanctifier-settings";

export interface AppSettings {
  analyzerPath: string;
  sanctifyToml: string;
  customRulesPath: string;
  apiEndpoint: string;
  aiProvider: "stub" | "anthropic" | "openai" | "local";
  aiApiKey: string;
  dailyCostCap: string;
  theme: string;
  telemetryEnabled: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  analyzerPath: "",
  sanctifyToml: "",
  customRulesPath: "",
  apiEndpoint: "",
  aiProvider: "stub",
  aiApiKey: "",
  dailyCostCap: "10.00",
  theme: "system",
  telemetryEnabled: false,
};

export function getSettings(): AppSettings {
  if (typeof window === "undefined") return DEFAULT_SETTINGS;
  try {
    const stored = localStorage.getItem(SETTINGS_KEY);
    if (stored) {
      return { ...DEFAULT_SETTINGS, ...JSON.parse(stored) };
    }
  } catch (error) {
    console.error("Failed to load settings:", error);
  }
  return DEFAULT_SETTINGS;
}

export function getSettingsHeaders(): Record<string, string> {
  const settings = getSettings();
  const headers: Record<string, string> = {};

  if (settings.analyzerPath) {
    headers["x-sanctifier-bin-path"] = settings.analyzerPath;
  }
  if (settings.sanctifyToml) {
    headers["x-sanctifier-toml"] = btoa(settings.sanctifyToml);
  }
  if (settings.customRulesPath) {
    headers["x-sanctifier-custom-rules"] = settings.customRulesPath;
  }
  if (settings.apiEndpoint) {
    headers["x-sanctifier-api-endpoint"] = settings.apiEndpoint;
  }
  if (settings.aiProvider !== "stub") {
    headers["x-sanctifier-ai-provider"] = settings.aiProvider;
    if (settings.aiApiKey) {
      headers["x-sanctifier-ai-key"] = settings.aiApiKey;
    }
  }
  headers["x-sanctifier-telemetry"] = settings.telemetryEnabled ? "1" : "0";

  return headers;
}
