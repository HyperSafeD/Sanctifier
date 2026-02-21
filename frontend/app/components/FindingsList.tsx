"use client";

import type { Finding, Severity } from "../types";
import { CodeSnippet } from "./CodeSnippet";

interface FindingsListProps {
  findings: Finding[];
  severityFilter: Severity | "all";
}

const severityColors: Record<Severity, string> = {
  critical: "bg-red-500/10 border-red-500/50 text-red-700 dark:text-red-400",
  high: "bg-orange-500/10 border-orange-500/50 text-orange-700 dark:text-orange-400",
  medium: "bg-amber-500/10 border-amber-500/50 text-amber-700 dark:text-amber-400",
  low: "bg-zinc-500/10 border-zinc-500/50 text-zinc-700 dark:text-zinc-400",
};

export function FindingsList({ findings, severityFilter }: FindingsListProps) {
  const filtered =
    severityFilter === "all"
      ? findings
      : findings.filter((f) => f.severity === severityFilter);

  return (
    <div className="space-y-4">
      {filtered.length === 0 ? (
        <p className="text-zinc-500 dark:text-zinc-400 py-8 text-center">
          No findings match the selected filter.
        </p>
      ) : (
        filtered.map((f) => (
          <div
            key={f.id}
            className={`rounded-lg border p-4 ${severityColors[f.severity]}`}
          >
            <div className="flex items-start justify-between gap-4">
              <div className="min-w-0 flex-1">
                <span className="text-xs font-semibold uppercase tracking-wide opacity-80">
                  {f.category}
                </span>
                <h3 className="mt-1 font-medium">{f.title}</h3>
                <p className="mt-1 text-sm opacity-90">{f.location}</p>
                {f.suggestion && (
                  <p className="mt-2 text-sm italic">💡 {f.suggestion}</p>
                )}
              </div>
              <span
                className={`shrink-0 rounded px-2 py-1 text-xs font-medium ${
                  severityColors[f.severity]
                }`}
              >
                {f.severity}
              </span>
            </div>
            {f.snippet && (
              <div className="mt-3">
                <CodeSnippet code={f.snippet} highlightLine={f.line} />
              </div>
            )}
          </div>
        ))
      )}
    </div>
  );
}
