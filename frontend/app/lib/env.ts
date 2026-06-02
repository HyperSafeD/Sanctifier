import "server-only";

type Status = "set" | "defaulted" | "missing";

type EnvEntry = {
  key: string;
  value: string | undefined;
  defaultValue?: string;
  description: string;
};

const entries: EnvEntry[] = [
  { key: "SANCTIFIER_BIN",                  value: process.env.SANCTIFIER_BIN,                  defaultValue: "sanctifier", description: "Path to the sanctifier binary" },
  { key: "AI_EXPLAIN_PROVIDER",             value: process.env.AI_EXPLAIN_PROVIDER,                             description: "AI provider (anthropic | openai)" },
  { key: "ANTHROPIC_API_KEY",               value: process.env.ANTHROPIC_API_KEY,               description: "Anthropic API key (required when provider=anthropic)" },
  { key: "OPENAI_API_KEY",                  value: process.env.OPENAI_API_KEY,                  description: "OpenAI API key (required when provider=openai)" },
  { key: "RATE_LIMIT_REQUESTS_PER_MINUTE",  value: process.env.RATE_LIMIT_REQUESTS_PER_MINUTE,  defaultValue: "10", description: "Max analyze requests per IP per minute" },
  { key: "API_KEYS",                         value: process.env.API_KEYS,                         description: "Comma-separated API keys for /api/v1/analyze endpoint" },
  { key: "API_RATE_LIMIT_PER_MINUTE",        value: process.env.API_RATE_LIMIT_PER_MINUTE,        defaultValue: "20", description: "Max v1 analyze requests per API key per minute" },
  { key: "AI_RATE_LIMIT_REQUESTS_PER_MINUTE", value: process.env.AI_RATE_LIMIT_REQUESTS_PER_MINUTE, defaultValue: "10", description: "Max AI explain requests per IP per minute" },
  { key: "AI_DAILY_COST_CAP_CENTS",          value: process.env.AI_DAILY_COST_CAP_CENTS,          defaultValue: "100", description: "Global daily AI cost cap in US cents" },
];

function statusOf(entry: EnvEntry): Status {
  if (entry.value?.trim()) return "set";
  if (entry.defaultValue !== undefined) return "defaulted";
  return "missing";
}

// Startup log table — emitted once at module load
const rows = entries.map((e) => {
  const status = statusOf(e);
  const icon = status === "set" ? "✓ set" : status === "defaulted" ? "! defaulted" : "✗ missing";
  return `  ${icon.padEnd(14)}${e.key.padEnd(36)}${e.description}`;
});
console.log(`\n[sanctifier] environment:\n${rows.join("\n")}\n`);

export const SANCTIFIER_BIN = process.env.SANCTIFIER_BIN?.trim() || "sanctifier";
export const AI_EXPLAIN_PROVIDER = process.env.AI_EXPLAIN_PROVIDER?.trim() || "";
export const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY || "";
export const OPENAI_API_KEY = process.env.OPENAI_API_KEY || "";
export const RATE_LIMIT_REQUESTS_PER_MINUTE = (() => {
  const raw = process.env.RATE_LIMIT_REQUESTS_PER_MINUTE;
  if (!raw) return 10;
  const parsed = parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 10;
})();

export const API_KEYS = (() => {
  const raw = process.env.API_KEYS;
  if (!raw) return [];
  return raw
    .split(",")
    .map((k) => k.trim())
    .filter(Boolean);
})();

export const API_RATE_LIMIT_PER_MINUTE = (() => {
  const raw = process.env.API_RATE_LIMIT_PER_MINUTE;
  if (!raw) return 20;
  const parsed = parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 20;
})();
