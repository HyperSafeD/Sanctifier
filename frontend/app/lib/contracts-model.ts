export type RiskLevel = "critical" | "high" | "medium" | "low" | "info";

export type ContractSortKey = "risk" | "findings" | "loc";

export interface ContractInfo {
  name: string;
  path: string;
  description: string;
  tags: string[];
  risk: RiskLevel;
  findingCount: number;
  linesOfCode: number;
}

export interface PatternCounts {
  panics: number;
  unsafeBlocks: number;
  rawInvokes: number;
}

export const RISK_ORDER: RiskLevel[] = ["critical", "high", "medium", "low", "info"];

const RISK_RANK: Record<RiskLevel, number> = {
  critical: 4,
  high: 3,
  medium: 2,
  low: 1,
  info: 0,
};

const TAG_KEYWORDS: Array<[string, RegExp]> = [
  ["token", /token|sep-?41/i],
  ["zk", /\bzk\b|groth16|proof|nullifier/i],
  ["governance", /governance|voting|dao/i],
  ["multisig", /multisig|multi-sig/i],
  ["upgradeable", /proxy|upgrade/i],
  ["oracle", /oracle/i],
  ["bridge", /bridge/i],
  ["vesting", /vesting|timelock/i],
  ["amm", /\bamm\b|liquidity|pool/i],
  ["flashloan", /flash-?loan/i],
  ["reentrancy", /reentrancy/i],
  ["vulnerable", /vulnerable|bugs|unsafe|shadowing|race/i],
];

/** Derive lightweight search tags from a contract's name and description. */
export function deriveTags(name: string, description: string, extras: string[] = []): string[] {
  const haystack = `${name} ${description}`;
  const tags = TAG_KEYWORDS.filter(([, re]) => re.test(haystack)).map(([tag]) => tag);
  return Array.from(new Set([...tags, ...extras]));
}

/**
 * Cheap source heuristics used for the explorer's pre-scan badges. Test modules
 * and line comments are ignored. A full `/scan` remains the authoritative result.
 */
export function countPatterns(source: string): PatternCounts {
  const testStart = source.indexOf("#[cfg(test)]");
  const body = testStart === -1 ? source : source.slice(0, testStart);
  const code = body
    .split("\n")
    .filter((line) => !line.trim().startsWith("//"))
    .join("\n");

  const count = (re: RegExp) => (code.match(re) ?? []).length;
  return {
    panics: count(/\bpanic!\s*\(|\.unwrap\s*\(\s*\)|\.expect\s*\(/g),
    unsafeBlocks: count(/\bunsafe\s*\{/g),
    rawInvokes: count(/\.invoke_contract\s*(::<[^>]*>)?\s*\(/g),
  };
}

export function totalFindings(counts: PatternCounts): number {
  return counts.panics + counts.unsafeBlocks + counts.rawInvokes;
}

export function riskFromCounts(counts: PatternCounts): RiskLevel {
  const weighted = counts.panics + counts.unsafeBlocks * 5 + counts.rawInvokes * 5;
  if (weighted >= 20) return "critical";
  if (weighted >= 10) return "high";
  if (weighted >= 4) return "medium";
  if (weighted >= 1) return "low";
  return "info";
}

export interface ContractFilters {
  query: string;
  risk: RiskLevel | "all";
  sort: ContractSortKey;
}

export const DEFAULT_CONTRACT_FILTERS: ContractFilters = {
  query: "",
  risk: "all",
  sort: "risk",
};

export function filterAndSortContracts(
  contracts: ContractInfo[],
  filters: ContractFilters,
): ContractInfo[] {
  const query = filters.query.trim().toLowerCase();

  const matches = contracts.filter((c) => {
    if (filters.risk !== "all" && c.risk !== filters.risk) return false;
    if (!query) return true;
    return (
      c.name.toLowerCase().includes(query) ||
      c.description.toLowerCase().includes(query) ||
      c.tags.some((t) => t.toLowerCase().includes(query))
    );
  });

  const compare: Record<ContractSortKey, (a: ContractInfo, b: ContractInfo) => number> = {
    risk: (a, b) => RISK_RANK[b.risk] - RISK_RANK[a.risk] || b.findingCount - a.findingCount,
    findings: (a, b) => b.findingCount - a.findingCount,
    loc: (a, b) => b.linesOfCode - a.linesOfCode,
  };

  return [...matches].sort((a, b) => compare[filters.sort](a, b) || a.name.localeCompare(b.name));
}
