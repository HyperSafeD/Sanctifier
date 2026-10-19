/**
 * SARIF 2.1.0 export / import utilities for Sanctifier findings.
 * Pure data transformation — no VS Code API dependency.
 */

import { EditorFinding, Severity } from './analyzer';

export const SARIF_SCHEMA = 'https://json.schemastore.org/sarif-2.1.0.json';
export const SARIF_VERSION = '2.1.0';

export interface SarifLocation {
  physicalLocation: {
    artifactLocation: { uri: string; uriBaseId?: string };
    region: { startLine: number; startColumn: number; endLine?: number; endColumn?: number };
  };
}

export interface SarifResult {
  ruleId: string;
  level: 'none' | 'note' | 'warning' | 'error';
  message: { text: string };
  locations: SarifLocation[];
}

export interface SarifRule {
  id: string;
  name: string;
  shortDescription: { text: string };
  defaultConfiguration: { level: 'none' | 'note' | 'warning' | 'error' };
}

export interface SarifRun {
  tool: {
    driver: {
      name: string;
      version: string;
      informationUri: string;
      rules: SarifRule[];
    };
  };
  results: SarifResult[];
}

export interface SarifLog {
  version: '2.1.0';
  $schema: string;
  runs: SarifRun[];
}

const RULE_META: Record<string, { name: string; description: string }> = {
  S001: { name: 'AuthGap', description: 'Missing require_auth on privileged operation' },
  S002: { name: 'PanicUsage', description: 'panic! aborts the contract — use structured errors' },
  S003: { name: 'ArithmeticOverflow', description: 'Unchecked arithmetic may overflow or underflow' },
  S006: { name: 'UnsafePattern', description: '.unwrap()/.expect() can abort the contract' },
};

function severityToSarifLevel(sev: Severity): 'none' | 'note' | 'warning' | 'error' {
  switch (sev) {
    case 'error': return 'error';
    case 'warning': return 'warning';
    case 'information': return 'note';
  }
}

function sarifLevelToSeverity(level: string): Severity {
  switch (level) {
    case 'error': return 'error';
    case 'note': return 'information';
    default: return 'warning';
  }
}

function buildRules(findings: EditorFinding[]): SarifRule[] {
  const seen = new Set<string>();
  const rules: SarifRule[] = [];
  for (const f of findings) {
    if (seen.has(f.code)) continue;
    seen.add(f.code);
    const meta = RULE_META[f.code] ?? { name: f.code, description: f.message };
    rules.push({
      id: f.code,
      name: meta.name,
      shortDescription: { text: meta.description },
      defaultConfiguration: { level: severityToSarifLevel(f.severity) },
    });
  }
  return rules;
}

/**
 * Converts an array of EditorFindings for a single file into a SARIF 2.1.0 log.
 */
export function findingsToSarif(findings: EditorFinding[], fileUri: string): SarifLog {
  const rules = buildRules(findings);
  const results: SarifResult[] = findings.map((f) => ({
    ruleId: f.code,
    level: severityToSarifLevel(f.severity),
    message: { text: f.message },
    locations: [
      {
        physicalLocation: {
          artifactLocation: { uri: fileUri, uriBaseId: '%SRCROOT%' },
          region: {
            startLine: f.line,
            startColumn: 1,
            ...(f.endLine !== undefined ? { endLine: f.endLine } : {}),
            ...(f.endCharacter !== undefined ? { endColumn: f.endCharacter } : {}),
          },
        },
      },
    ],
  }));

  return {
    version: SARIF_VERSION,
    $schema: SARIF_SCHEMA,
    runs: [
      {
        tool: {
          driver: {
            name: 'sanctifier',
            version: '0.1.0',
            informationUri: 'https://github.com/HyperSafeD/Sanctifier',
            rules,
          },
        },
        results,
      },
    ],
  };
}

/**
 * Parses a SARIF 2.1.0 log back into EditorFindings.
 * Only processes runs from the sanctifier driver.
 */
export function sarifToFindings(log: SarifLog): EditorFinding[] {
  const findings: EditorFinding[] = [];
  for (const run of log.runs ?? []) {
    const results = run.results ?? [];
    for (const result of results) {
      const loc = result.locations?.[0]?.physicalLocation?.region;
      if (!loc) continue;
      findings.push({
        line: loc.startLine ?? 1,
        code: result.ruleId ?? 'UNKNOWN',
        severity: sarifLevelToSeverity(result.level),
        message: result.message?.text ?? '',
        ...(loc.endLine !== undefined ? { endLine: loc.endLine } : {}),
        ...(loc.endColumn !== undefined ? { endCharacter: loc.endColumn } : {}),
      });
    }
  }
  return findings;
}

/**
 * Serialises a SARIF log to a formatted JSON string.
 */
export function serialiseSarif(log: SarifLog): string {
  return JSON.stringify(log, null, 2);
}

/**
 * Parses a JSON string into a SarifLog. Throws on invalid JSON.
 * Caller is responsible for validating the result shape with validateSarifShape().
 */
export function parseSarif(raw: string): SarifLog {
  return JSON.parse(raw) as SarifLog;
}

/**
 * Lightweight structural validation — confirms mandatory SARIF 2.1.0 fields exist.
 */
export function validateSarifShape(obj: unknown): obj is SarifLog {
  if (typeof obj !== 'object' || obj === null) return false;
  const o = obj as Record<string, unknown>;
  if (o['version'] !== SARIF_VERSION) return false;
  if (!Array.isArray(o['runs'])) return false;
  return true;
}
