import type { Finding } from "../../../types";

// ── Cache ──────────────────────────────────────────────────────────────────────
const explanationCache = new Map<string, { explanation: string; fixCode: string }>();

function cacheKey(finding: Finding): string {
  return `${finding.code}|${(finding.snippet || "").slice(0, 200)}`;
}

// ── Rate limiter (per-IP) ──────────────────────────────────────────────────────
const ipRequests = new Map<string, number[]>();
const AI_RATE_LIMIT = parseInt(process.env.AI_RATE_LIMIT_REQUESTS_PER_MINUTE || "10", 10);

function checkRateLimit(ip: string): boolean {
  const now = Date.now();
  const window = now - 60_000;
  let timestamps = ipRequests.get(ip) || [];
  timestamps = timestamps.filter((t) => t > window);
  if (timestamps.length >= AI_RATE_LIMIT) return false;
  timestamps.push(now);
  ipRequests.set(ip, timestamps);
  return true;
}

// ── Global daily cost cap ──────────────────────────────────────────────────────
const DAILY_COST_CAP_CENTS = parseInt(process.env.AI_DAILY_COST_CAP_CENTS || "100", 10);
let dailyCostCents = 0;
let dailyCostReset = Date.now() + 86_400_000;

function checkCostCap(costCents: number): boolean {
  if (Date.now() > dailyCostReset) {
    dailyCostCents = 0;
    dailyCostReset = Date.now() + 86_400_000;
  }
  return dailyCostCents + costCents <= DAILY_COST_CAP_CENTS;
}

function addCost(costCents: number): void {
  dailyCostCents += costCents;
}

// ── Provider interface ─────────────────────────────────────────────────────────
export interface AiProvider {
  name: string;
  explain(finding: Finding): Promise<{ explanation: string; fixCode: string }>;
  explainStream?(finding: Finding): AsyncGenerator<string>;
}

// ── LocalStubProvider ─────────────────────────────────────────────────────────
class LocalStubProvider implements AiProvider {
  name = "stub";

  async explain(finding: Finding): Promise<{ explanation: string; fixCode: string }> {
    await new Promise((r) => setTimeout(r, 200));
    return this.lookup(finding);
  }

  private lookup(finding: Finding): { explanation: string; fixCode: string } {
    const stubs: Record<string, { explanation: string; fixCode: string }> = {
      "auth-gap": {
        explanation:
          "This function lacks caller authentication. Add `address.require_auth()` to restrict access.",
        fixCode: `pub fn sensitive_action(env: Env, user: Address) {\n    user.require_auth();\n    // ...\n}`,
      },
      "arithmetic-overflow": {
        explanation:
          "Unchecked arithmetic may overflow. Use checked operations like `checked_add`.",
        fixCode: `let result = a.checked_add(b).ok_or(Error::Overflow)?;`,
      },
      "storage-collision": {
        explanation:
          "Potential storage key collision. Use a typed enum to disambiguate keys.",
        fixCode: `#[repr(u32)]\npub enum DataKey { Admin = 1, Balance(Address) = 2 }`,
      },
    };
    const cat = (finding.category || "").toLowerCase();
    return stubs[cat] || {
      explanation: `The finding "${finding.title}" at ${finding.location} may indicate a ${finding.category} risk. Review the logic and apply Soroban best practices.`,
      fixCode: finding.suggestion
        ? `// Suggested fix:\n// ${finding.suggestion}`
        : "// Review the specified location and add necessary checks.",
    };
  }
}

// ── OpenAIProvider ─────────────────────────────────────────────────────────────
class OpenAIProvider implements AiProvider {
  name = "openai";

  async explain(finding: Finding): Promise<{ explanation: string; fixCode: string }> {
    const apiKey = process.env.OPENAI_API_KEY;
    if (!apiKey) throw new Error("OPENAI_API_KEY not configured");

    const resp = await fetch("https://api.openai.com/v1/chat/completions", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: "gpt-4o-mini",
        messages: [
          { role: "system", content: "You are a Soroban smart contract security expert." },
          {
            role: "user",
            content: `Explain this finding and suggest a code fix:\nCode: ${finding.code}\nCategory: ${finding.category}\nTitle: ${finding.title}\nLocation: ${finding.location}\nSnippet: ${(finding.snippet || "").slice(0, 1000)}\nSuggestion: ${finding.suggestion || "N/A"}\n\nRespond as JSON with keys: explanation (string), fixCode (string).`,
          },
        ],
        temperature: 0.3,
        max_tokens: 1000,
      }),
    });

    if (!resp.ok) {
      const body = await resp.text();
      throw new Error(`OpenAI API error ${resp.status}: ${body}`);
    }

    const data = await resp.json();
    const text = data.choices?.[0]?.message?.content || "";

    try {
      return JSON.parse(text);
    } catch {
      return {
        explanation: text,
        fixCode: "// See explanation above for suggested fixes.",
      };
    }
  }

  async *explainStream(finding: Finding): AsyncGenerator<string> {
    const apiKey = process.env.OPENAI_API_KEY;
    if (!apiKey) throw new Error("OPENAI_API_KEY not configured");

    const resp = await fetch("https://api.openai.com/v1/chat/completions", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: "gpt-4o-mini",
        messages: [
          { role: "system", content: "You are a Soroban smart contract security expert." },
          {
            role: "user",
            content: `Explain this finding and suggest a code fix:\nCode: ${finding.code}\nCategory: ${finding.category}\nTitle: ${finding.title}\nLocation: ${finding.location}\nSnippet: ${(finding.snippet || "").slice(0, 1000)}\nSuggestion: ${finding.suggestion || "N/A"}`,
          },
        ],
        temperature: 0.3,
        max_tokens: 1000,
        stream: true,
      }),
    });

    if (!resp.ok) {
      throw new Error(`OpenAI API error ${resp.status}`);
    }

    const reader = resp.body?.getReader();
    if (!reader) throw new Error("No response body");

    const decoder = new TextDecoder();
    let buffer = "";
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";
      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed || trimmed === "data: [DONE]") continue;
        if (trimmed.startsWith("data: ")) {
          try {
            const parsed = JSON.parse(trimmed.slice(6));
            const content = parsed.choices?.[0]?.delta?.content || "";
            if (content) yield content;
          } catch {
            // skip parse errors
          }
        }
      }
    }
  }
}

// ── AnthropicProvider ──────────────────────────────────────────────────────────
class AnthropicProvider implements AiProvider {
  name = "anthropic";

  async explain(finding: Finding): Promise<{ explanation: string; fixCode: string }> {
    const apiKey = process.env.ANTHROPIC_API_KEY;
    if (!apiKey) throw new Error("ANTHROPIC_API_KEY not configured");

    const resp = await fetch("https://api.anthropic.com/v1/messages", {
      method: "POST",
      headers: {
        "x-api-key": apiKey,
        "anthropic-version": "2023-06-01",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: "claude-3-haiku-20240307",
        max_tokens: 1000,
        messages: [
          {
            role: "user",
            content: `You are a Soroban smart contract security expert. Explain this finding and suggest a code fix:\n\nCode: ${finding.code}\nCategory: ${finding.category}\nTitle: ${finding.title}\nLocation: ${finding.location}\nSnippet: ${(finding.snippet || "").slice(0, 1000)}\nSuggestion: ${finding.suggestion || "N/A"}\n\nRespond as JSON with keys: explanation (string), fixCode (string).`,
          },
        ],
      }),
    });

    if (!resp.ok) {
      const body = await resp.text();
      throw new Error(`Anthropic API error ${resp.status}: ${body}`);
    }

    const data = await resp.json();
    const text = data.content?.[0]?.text || "";

    try {
      return JSON.parse(text);
    } catch {
      return {
        explanation: text,
        fixCode: "// See explanation above for suggested fixes.",
      };
    }
  }

  async *explainStream(finding: Finding): AsyncGenerator<string> {
    const apiKey = process.env.ANTHROPIC_API_KEY;
    if (!apiKey) throw new Error("ANTHROPIC_API_KEY not configured");

    const resp = await fetch("https://api.anthropic.com/v1/messages", {
      method: "POST",
      headers: {
        "x-api-key": apiKey,
        "anthropic-version": "2023-06-01",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: "claude-3-haiku-20240307",
        max_tokens: 1000,
        stream: true,
        messages: [
          {
            role: "user",
            content: `You are a Soroban smart contract security expert. Explain this finding and suggest a code fix:\n\nCode: ${finding.code}\nCategory: ${finding.category}\nTitle: ${finding.title}\nLocation: ${finding.location}\nSnippet: ${(finding.snippet || "").slice(0, 1000)}\nSuggestion: ${finding.suggestion || "N/A"}`,
          },
        ],
      }),
    });

    if (!resp.ok) throw new Error(`Anthropic API error ${resp.status}`);

    const reader = resp.body?.getReader();
    if (!reader) throw new Error("No response body");

    const decoder = new TextDecoder();
    let buffer = "";
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";
      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed || !trimmed.startsWith("data: ")) continue;
        try {
          const parsed = JSON.parse(trimmed.slice(6));
          if (parsed.type === "content_block_delta" && parsed.delta?.text) {
            yield parsed.delta.text;
          }
        } catch {
          // skip parse errors
        }
      }
    }
  }
}

// ── Factory ────────────────────────────────────────────────────────────────────
export function getProvider(): AiProvider {
  const name = (process.env.AI_EXPLAIN_PROVIDER || "stub").toLowerCase();
  switch (name) {
    case "openai":
      return new OpenAIProvider();
    case "anthropic":
      return new AnthropicProvider();
    default:
      return new LocalStubProvider();
  }
}

// ── Public API ─────────────────────────────────────────────────────────────────
export async function getExplanation(
  finding: Finding,
  ip: string,
): Promise<{ explanation: string; fixCode: string; cached: boolean }> {
  const key = cacheKey(finding);
  const cached = explanationCache.get(key);
  if (cached) return { ...cached, cached: true };

  if (!checkRateLimit(ip)) {
    throw new Error("Rate limit exceeded. Try again later.");
  }

  const costEstimate = 1; // ~$0.01 per request
  if (!checkCostCap(costEstimate)) {
    throw new Error("Daily cost cap reached. Try again tomorrow.");
  }

  const provider = getProvider();
  const result = await provider.explain(finding);
  addCost(costEstimate);

  explanationCache.set(key, result);
  if (explanationCache.size > 500) {
    const firstKey = explanationCache.keys().next().value;
    if (firstKey) explanationCache.delete(firstKey);
  }

  return { ...result, cached: false };
}

export async function* streamExplanation(
  finding: Finding,
  ip: string,
): AsyncGenerator<string> {
  if (!checkRateLimit(ip)) {
    throw new Error("Rate limit exceeded. Try again later.");
  }

  const provider = getProvider();
  if (!provider.explainStream) {
    throw new Error("Streaming not supported by current provider");
  }

  yield* provider.explainStream(finding);
}
