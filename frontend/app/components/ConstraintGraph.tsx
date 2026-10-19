"use client";

import { useMemo, useState } from "react";
import type { Finding } from "../types";

// ── Types ─────────────────────────────────────────────────────────────────────

export interface CircuitSignal {
  id: string;
  name: string;
  /** "input" | "output" | "intermediate" */
  kind: "input" | "output" | "intermediate";
  /** ZK finding codes that flag this signal (e.g. ["Z001", "Z007"]) */
  flaggedBy?: string[];
}

export interface CircuitConstraint {
  id: string;
  /** Human-readable label for the constraint, e.g. "a * b === c" */
  label: string;
  /** Signal IDs involved */
  signals: string[];
  /** Finding codes flagging this constraint */
  flaggedBy?: string[];
}

export interface CircuitGraph {
  signals: CircuitSignal[];
  constraints: CircuitConstraint[];
}

interface ConstraintGraphProps {
  graph: CircuitGraph;
  /** Findings from the ZK analysis, used to highlight flagged nodes */
  findings?: Finding[];
  className?: string;
}

// ── Layout ────────────────────────────────────────────────────────────────────

const COL_X: Record<CircuitSignal["kind"], number> = {
  input: 60,
  intermediate: 260,
  output: 460,
};

const NODE_R = 22;
const ROW_GAP = 70;

interface LayoutSignal extends CircuitSignal {
  x: number;
  y: number;
}

function layoutSignals(signals: CircuitSignal[]): LayoutSignal[] {
  const counters: Record<string, number> = { input: 0, intermediate: 0, output: 0 };
  return signals.map((s) => {
    const row = counters[s.kind]++;
    return { ...s, x: COL_X[s.kind], y: 60 + row * ROW_GAP };
  });
}

// ── Colors ────────────────────────────────────────────────────────────────────

const KIND_COLOR: Record<CircuitSignal["kind"], { fill: string; stroke: string; dark: string }> = {
  input:        { fill: "#dbeafe", stroke: "#3b82f6", dark: "#1e3a5f" },
  intermediate: { fill: "#f3f4f6", stroke: "#6b7280", dark: "#27272a" },
  output:       { fill: "#d1fae5", stroke: "#10b981", dark: "#052e16" },
};

const ZK_FLAG_COLOR = "#8b5cf6"; // violet — matches ZkBadge in FindingsList

// ── Component ─────────────────────────────────────────────────────────────────

/**
 * Interactive constraint-graph visualization for ZK circuits (issue #1236).
 *
 * Renders circuit signals as nodes (grouped by input / intermediate / output)
 * with constraints shown as edges. Signals flagged by Z-rule findings are
 * highlighted in violet so developers can immediately spot problem areas.
 */
export function ConstraintGraph({ graph, findings = [], className = "" }: ConstraintGraphProps) {
  const [hovered, setHovered] = useState<string | null>(null);
  const [selectedSignal, setSelectedSignal] = useState<string | null>(null);

  // Build a set of flagged signal IDs from findings + explicit flaggedBy fields.
  const flaggedSignalIds = useMemo<Set<string>>(() => {
    const s = new Set<string>();
    for (const sig of graph.signals) {
      if (sig.flaggedBy && sig.flaggedBy.length > 0) s.add(sig.id);
    }
    // Also flag signals whose names appear in finding locations/titles.
    for (const f of findings) {
      if (/^Z\d+$/.test(f.code)) {
        for (const sig of graph.signals) {
          if (f.title.includes(sig.name) || (f.location ?? "").includes(sig.name)) {
            s.add(sig.id);
          }
        }
      }
    }
    return s;
  }, [graph.signals, findings]);

  const layouted = useMemo(() => layoutSignals(graph.signals), [graph.signals]);
  const byId = useMemo(
    () => new Map(layouted.map((s) => [s.id, s])),
    [layouted],
  );

  const svgHeight = useMemo(() => {
    const maxRows = Math.max(
      graph.signals.filter((s) => s.kind === "input").length,
      graph.signals.filter((s) => s.kind === "intermediate").length,
      graph.signals.filter((s) => s.kind === "output").length,
      1,
    );
    return 60 + maxRows * ROW_GAP + 40;
  }, [graph.signals]);

  const selectedInfo = selectedSignal ? byId.get(selectedSignal) : null;

  if (graph.signals.length === 0) {
    return (
      <div className={`flex items-center justify-center rounded-xl border border-dashed border-zinc-300 dark:border-zinc-700 p-12 text-zinc-400 dark:text-zinc-500 ${className}`}>
        No circuit signals to display.
      </div>
    );
  }

  return (
    <div className={`flex flex-col gap-4 ${className}`}>
      {/* Legend */}
      <div className="flex flex-wrap items-center gap-4 text-xs text-zinc-500 dark:text-zinc-400">
        {(["input", "intermediate", "output"] as const).map((kind) => (
          <span key={kind} className="flex items-center gap-1.5">
            <span
              className="inline-block h-3 w-3 rounded-full border-2"
              style={{ background: KIND_COLOR[kind].fill, borderColor: KIND_COLOR[kind].stroke }}
            />
            {kind.charAt(0).toUpperCase() + kind.slice(1)} signal
          </span>
        ))}
        <span className="flex items-center gap-1.5">
          <span className="inline-block h-3 w-3 rounded-full border-2" style={{ background: "#ede9fe", borderColor: ZK_FLAG_COLOR }} />
          Flagged by Z-rule
        </span>
      </div>

      {/* SVG graph */}
      <div className="overflow-x-auto rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 shadow">
        <svg
          viewBox={`0 0 600 ${svgHeight}`}
          width="100%"
          style={{ minWidth: 480, minHeight: svgHeight }}
          role="img"
          aria-label="Circuit constraint graph"
        >
          {/* Column labels */}
          {(["input", "intermediate", "output"] as const).map((kind) => (
            <text
              key={kind}
              x={COL_X[kind]}
              y={28}
              textAnchor="middle"
              fontSize={11}
              fontWeight={600}
              fill="#6b7280"
              fontFamily="monospace"
            >
              {kind.toUpperCase()}
            </text>
          ))}

          {/* Constraint edges */}
          {graph.constraints.map((c) => {
            const involved = c.signals.map((id) => byId.get(id)).filter(Boolean) as LayoutSignal[];
            if (involved.length < 2) return null;
            const isFlagged = (c.flaggedBy?.length ?? 0) > 0;
            const color = isFlagged ? ZK_FLAG_COLOR : "#d1d5db";
            // Draw lines between consecutive pairs of involved signals.
            return involved.slice(0, -1).map((a, idx) => {
              const b = involved[idx + 1];
              return (
                <line
                  key={`${c.id}-${idx}`}
                  x1={a.x}
                  y1={a.y}
                  x2={b.x}
                  y2={b.y}
                  stroke={color}
                  strokeWidth={isFlagged ? 2 : 1.5}
                  strokeDasharray={isFlagged ? "5 3" : undefined}
                  opacity={0.7}
                />
              );
            });
          })}

          {/* Signal nodes */}
          {layouted.map((sig) => {
            const flagged = flaggedSignalIds.has(sig.id);
            const isHovered = hovered === sig.id;
            const isSelected = selectedSignal === sig.id;
            const colors = KIND_COLOR[sig.kind];
            return (
              <g
                key={sig.id}
                onClick={() => setSelectedSignal(isSelected ? null : sig.id)}
                onMouseEnter={() => setHovered(sig.id)}
                onMouseLeave={() => setHovered(null)}
                style={{ cursor: "pointer" }}
                role="button"
                aria-pressed={isSelected}
                aria-label={`Signal: ${sig.name}${flagged ? " (flagged)" : ""}`}
              >
                {/* Outer glow for flagged signals */}
                {flagged && (
                  <circle
                    cx={sig.x}
                    cy={sig.y}
                    r={NODE_R + 6}
                    fill={ZK_FLAG_COLOR}
                    opacity={0.15}
                  />
                )}
                <circle
                  cx={sig.x}
                  cy={sig.y}
                  r={NODE_R}
                  fill={flagged ? "#ede9fe" : colors.fill}
                  stroke={flagged ? ZK_FLAG_COLOR : isHovered || isSelected ? colors.stroke : colors.stroke}
                  strokeWidth={flagged ? 2.5 : isHovered || isSelected ? 2 : 1.5}
                />
                <text
                  x={sig.x}
                  y={sig.y + 1}
                  textAnchor="middle"
                  dominantBaseline="middle"
                  fontSize={9}
                  fontWeight={600}
                  fontFamily="monospace"
                  fill={flagged ? ZK_FLAG_COLOR : "#374151"}
                >
                  {sig.name.length > 8 ? sig.name.slice(0, 7) + "…" : sig.name}
                </text>
                {/* ZK flag indicator dot */}
                {flagged && (
                  <circle cx={sig.x + NODE_R - 4} cy={sig.y - NODE_R + 4} r={5} fill={ZK_FLAG_COLOR} stroke="#fff" strokeWidth={1} />
                )}
              </g>
            );
          })}
        </svg>
      </div>

      {/* Detail panel for selected signal */}
      {selectedInfo && (
        <div className="rounded-xl border border-violet-200 dark:border-violet-800 bg-violet-50 dark:bg-violet-950/30 p-4 text-sm">
          <p className="font-semibold text-violet-800 dark:text-violet-200 font-mono">{selectedInfo.name}</p>
          <p className="text-violet-600 dark:text-violet-400 capitalize mt-0.5">{selectedInfo.kind} signal</p>
          {flaggedSignalIds.has(selectedInfo.id) && selectedInfo.flaggedBy && selectedInfo.flaggedBy.length > 0 && (
            <p className="mt-2 text-violet-700 dark:text-violet-300">
              Flagged by: <span className="font-mono font-bold">{selectedInfo.flaggedBy.join(", ")}</span>
            </p>
          )}
          {!flaggedSignalIds.has(selectedInfo.id) && (
            <p className="mt-2 text-zinc-500 dark:text-zinc-400">No Z-rule findings for this signal.</p>
          )}
        </div>
      )}
    </div>
  );
}
