import type { Finding, Severity } from "../types";
import { errorMessages } from "./error-messages";

const LEGACY_CODE_MAP: Record<string, string> = {
  AUTH_GAP: "S001",
  PANIC_USAGE: "S002",
  ARITHMETIC_OVERFLOW: "S003",
  LEDGER_SIZE_RISK: "S004",
  STORAGE_COLLISION: "S005",
  UNSAFE_PATTERN: "S006",
  CUSTOM_RULE: "S007",
  CUSTOM_RULE_MATCH: "S007",
  EVENT_INCONSISTENCY: "S008",
  UNHANDLED_RESULT: "S009",
  UPGRADE_RISK: "S010",
  SMT_INVARIANT_VIOLATION: "S011",
  SEP41_INTERFACE_DEVIATION: "S012",
};

export const FINDING_CODE_PATTERN = /^S\d{3}$/;

export function canonicalizeFindingCode(code: string): string {
  const normalized = code.trim().toUpperCase();
  if (!normalized) {
    return "";
  }

  if (FINDING_CODE_PATTERN.test(normalized)) {
    return normalized;
  }

  return LEGACY_CODE_MAP[normalized] ?? normalized;
}

export function normalizeFindingCodeQuery(input: string): string {
  return input.trim().toUpperCase();
}

export function validateFindingCodeQuery(query: string): string | null {
  if (!query) {
    return null;
  }

  if (!FINDING_CODE_PATTERN.test(query)) {
    return errorMessages.findingCode.invalidFormat;
  }

  return null;
}

export function filterFindings(
  findings: Finding[],
  severityFilter: Severity | "all",
  codeFilter: string
): Finding[] {
  return findings.filter((finding) => {
    if (severityFilter !== "all" && finding.severity !== severityFilter) {
      return false;
    }

    if (codeFilter && canonicalizeFindingCode(finding.code) !== codeFilter) {
      return false;
    }

    return true;
  });
}
