"use client";

import { ExternalLink, ShieldAlert, ZapOff } from "lucide-react";
import type { Finding } from "../types";

// Feature-flagged via NEXT_PUBLIC_ZK_ENABLED env var (issue #1142).
// The panel renders only when the flag is truthy so non-ZK projects
// never see it in their scan results.
export const ZK_ENABLED =
  typeof process !== "undefined" &&
  process.env.NEXT_PUBLIC_ZK_ENABLED === "true";

const RULE_DOCS_BASE =
  "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/rules/";

/** Z-rule codes that carry ZK-specific metadata fields. */
const ZK_RULE_CODES = new Set([
  "Z001", "Z002", "Z003", "Z004", "Z005",
  "Z006", "Z007", "Z008", "Z009", "Z010",
]);

function isZkFinding(f: Finding): boolean {
  return ZK_RULE_CODES.has(f.code) || f.category === "zk";
}

interface ZkFindingCardProps {
  finding: Finding;
}

function ZkFindingCard({ finding }: ZkFindingCardProps) {
  const meta = finding.raw as Record<string, unknown> | null;
  const circuitFile   = meta?.circuit_file   as string | undefined;
  const signalName    = meta?.signal_name    as string | undefined;
  const templateName  = meta?.template_name  as string | undefined;
  const constraintRef = meta?.constraint_ref as string | undefined;
  const docsUrl = `${RULE_DOCS_BASE}${finding.code}.md`;

  const severityBg: Record<string, string> = {
    critical: "border-red-500/60 bg-red-500/8",
    high:     "border-orange-500/60 bg-orange-500/8",
    medium:   "border-amber-500/60 bg-amber-500/8",
    low:      "border-zinc-400/40 bg-zinc-500/5",
  };

  const severityBadge: Record<string, string> = {
    critical: "bg-red-500/15 text-red-600 dark:text-red-400",
    high:     "bg-orange-500/15 text-orange-600 dark:text-orange-400",
    medium:   "bg-amber-500/15 text-amber-600 dark:text-amber-400",
    low:      "bg-zinc-500/15 text-zinc-600 dark:text-zinc-400",
  };

  return (
    <div
      className={`rounded-lg border p-4 space-y-2 ${severityBg[finding.severity] ?? severityBg.low}`}
    >
      {/* Header row */}
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2 flex-wrap">
            <span
              className={`inline-block px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wide ${severityBadge[finding.severity] ?? severityBadge.low}`}
            >
              {finding.severity}
            </span>
            <span className="font-mono text-xs text-zinc-500 dark:text-zinc-400">
              {finding.code}
            </span>
            <a
              href={docsUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-[10px] text-blue-600 dark:text-blue-400 hover:underline"
              aria-label={`Documentation for ${finding.code}`}
            >
              docs <ExternalLink size={10} />
            </a>
          </div>
          <h3 className="mt-1 text-sm font-semibold leading-snug">{finding.title}</h3>
        </div>
        <ShieldAlert
          size={18}
          className="flex-shrink-0 text-zinc-400 dark:text-zinc-500 mt-0.5"
          aria-hidden
        />
      </div>

      {/* Circuit context — ZK-specific fields */}
      {(circuitFile || signalName || templateName || constraintRef) && (
        <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
          {circuitFile && (
            <>
              <dt className="text-zinc-500 dark:text-zinc-400">Circuit file</dt>
              <dd className="font-mono truncate" title={circuitFile}>{circuitFile}</dd>
            </>
          )}
          {templateName && (
            <>
              <dt className="text-zinc-500 dark:text-zinc-400">Template</dt>
              <dd className="font-mono truncate">{templateName}</dd>
            </>
          )}
          {signalName && (
            <>
              <dt className="text-zinc-500 dark:text-zinc-400">Signal</dt>
              <dd className="font-mono truncate">{signalName}</dd>
            </>
          )}
          {constraintRef && (
            <>
              <dt className="text-zinc-500 dark:text-zinc-400">Constraint</dt>
              <dd className="font-mono truncate">{constraintRef}</dd>
            </>
          )}
        </dl>
      )}

      {/* Location + suggestion */}
      {finding.location && (
        <p className="text-xs text-zinc-500 dark:text-zinc-400 font-mono truncate">
          {finding.location}
          {finding.line !== undefined && `:${finding.line}`}
        </p>
      )}
      {finding.suggestion && (
        <p className="text-xs text-zinc-600 dark:text-zinc-300 border-l-2 border-zinc-300 dark:border-zinc-600 pl-2">
          {finding.suggestion}
        </p>
      )}
    </div>
  );
}

interface ZkFindingsPanelProps {
  findings: Finding[];
}

/**
 * ZkFindingsPanel — renders Z-rule findings with circuit-appropriate context.
 *
 * Gated behind `NEXT_PUBLIC_ZK_ENABLED=true` (issue #1142 feature flag).
 * Non-ZK findings in the `findings` array are ignored by this panel; the
 * standard FindingsList handles them.
 */
export function ZkFindingsPanel({ findings }: ZkFindingsPanelProps) {
  if (!ZK_ENABLED) return null;

  const zkFindings = findings.filter(isZkFinding);
  if (zkFindings.length === 0) return null;

  return (
    <section aria-labelledby="zk-findings-heading" className="space-y-3">
      {/* Panel header */}
      <div className="flex items-center gap-2">
        <ZapOff size={16} className="text-purple-500" aria-hidden />
        <h2
          id="zk-findings-heading"
          className="text-sm font-semibold text-zinc-800 dark:text-zinc-200"
        >
          ZK Findings
          <span className="ml-2 text-xs font-normal text-zinc-500 dark:text-zinc-400">
            ({zkFindings.length})
          </span>
        </h2>
        <a
          href={`${RULE_DOCS_BASE}Z001.md`}
          target="_blank"
          rel="noopener noreferrer"
          className="ml-auto text-[10px] text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1"
        >
          Z-rule docs <ExternalLink size={10} />
        </a>
      </div>

      {/* Finding cards */}
      <div className="space-y-2">
        {zkFindings.map((f) => (
          <ZkFindingCard key={f.id} finding={f} />
        ))}
      </div>
    </section>
  );
}
