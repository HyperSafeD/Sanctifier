export type PaletteCategory = "Page" | "Contract" | "Finding code" | "Setting";

export interface PaletteItem {
  id: string;
  label: string;
  description?: string;
  category: PaletteCategory;
  /** Internal route to navigate to, or an absolute URL for external references. */
  href?: string;
  /** Extra text that should match searches but is not displayed. */
  keywords?: string;
  /** Named in-app action, resolved by the palette component (e.g. "toggle-theme"). */
  action?: "toggle-theme";
}

const DOCS_URL = "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md";

export const PAGE_ITEMS: PaletteItem[] = [
  { id: "page:home", label: "Home", category: "Page", href: "/", keywords: "landing" },
  { id: "page:scan", label: "Scan", category: "Page", href: "/scan", description: "Upload and analyze a contract" },
  { id: "page:dashboard", label: "Dashboard", category: "Page", href: "/dashboard", description: "Findings, call graph, diff" },
  { id: "page:contracts", label: "Contracts Explorer", category: "Page", href: "/contracts", description: "Browse workspace contracts" },
  { id: "page:audit", label: "Audit Report", category: "Page", href: "/audit", description: "Workspace summary with PDF/SARIF export" },
  { id: "page:playground", label: "Playground", category: "Page", href: "/playground" },
  { id: "page:terminal", label: "Terminal", category: "Page", href: "/terminal" },
  { id: "page:terms", label: "Terms of Service", category: "Page", href: "/terms" },
  { id: "page:privacy", label: "Privacy Policy", category: "Page", href: "/privacy" },
];

export const SETTING_ITEMS: PaletteItem[] = [
  { id: "setting:settings", label: "Settings", category: "Setting", href: "/settings", description: "Analysis and API preferences" },
  { id: "setting:theme", label: "Toggle theme", category: "Setting", action: "toggle-theme", description: "Cycle light, dark, system, high contrast", keywords: "dark mode appearance" },
  { id: "setting:webhooks", label: "Webhooks", category: "Setting", href: "/dashboard/webhooks", description: "Configure notifications" },
];

const FINDING_CODES: Array<[string, string]> = [
  ["S001", "Missing require_auth in a state-changing function"],
  ["S002", "panic! / unwrap / expect usage that may abort execution"],
  ["S003", "Unchecked arithmetic with overflow/underflow risk"],
  ["S004", "Ledger entry size exceeds or approaches the threshold"],
  ["S005", "Potential storage-key collision across data paths"],
  ["S006", "Potentially unsafe language or runtime pattern"],
  ["S007", "User-defined rule matched contract source"],
  ["S008", "Inconsistent topic counts or sub-optimal event patterns"],
  ["S009", "A Result return value is not consumed or handled"],
  ["S010", "Security risk in contract upgrade or admin mechanisms"],
  ["S011", "Z3 proved a violation of an invariant"],
  ["S012", "SEP-41 token interface deviation"],
  ["S022", "Raw invoke_contract call that panics on callee failure"],
];

export const FINDING_CODE_ITEMS: PaletteItem[] = FINDING_CODES.map(([code, meaning]) => ({
  id: `code:${code}`,
  label: code,
  description: meaning,
  category: "Finding code",
  href: DOCS_URL,
}));

export function contractItem(name: string, description: string): PaletteItem {
  return {
    id: `contract:${name}`,
    label: name,
    description,
    category: "Contract",
    href: `/contracts/${encodeURIComponent(name)}`,
  };
}

/** Score an item against a query; 0 means no match. Prefix and word-start matches rank higher. */
export function scoreItem(item: PaletteItem, query: string): number {
  const q = query.trim().toLowerCase();
  if (!q) return 1;

  const label = item.label.toLowerCase();
  if (label === q) return 100;
  if (label.startsWith(q)) return 80;
  if (label.split(/[\s\-_/]+/).some((w) => w.startsWith(q))) return 60;
  if (label.includes(q)) return 40;

  const rest = `${item.description ?? ""} ${item.keywords ?? ""} ${item.category}`.toLowerCase();
  return rest.includes(q) ? 20 : 0;
}

export function searchItems(items: PaletteItem[], query: string, limit = 30): PaletteItem[] {
  return items
    .map((item) => ({ item, score: scoreItem(item, query) }))
    .filter((r) => r.score > 0)
    .sort((a, b) => b.score - a.score || a.item.label.localeCompare(b.item.label))
    .slice(0, limit)
    .map((r) => r.item);
}

// ── Recent items (per-browser, SSR-safe) ─────────────────────────────────────

export const RECENT_STORAGE_KEY = "sanctifier_palette_recent";
export const MAX_RECENT = 5;

export function readRecentIds(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const parsed = JSON.parse(window.localStorage.getItem(RECENT_STORAGE_KEY) ?? "[]");
    return Array.isArray(parsed) ? parsed.filter((v): v is string => typeof v === "string") : [];
  } catch {
    return [];
  }
}

export function pushRecentId(id: string): string[] {
  const next = [id, ...readRecentIds().filter((existing) => existing !== id)].slice(0, MAX_RECENT);
  if (typeof window !== "undefined") {
    try {
      window.localStorage.setItem(RECENT_STORAGE_KEY, JSON.stringify(next));
    } catch {
      // storage unavailable or full — recents are best-effort
    }
  }
  return next;
}
