"use client";

import { useMemo, useState, memo } from "react";
import type { CallGraphNode, CallGraphEdge } from "../types";

interface CallGraphProps {
  nodes: CallGraphNode[];
  edges: CallGraphEdge[];
}

const RENDER_THRESHOLD = 100;

const NODE_COLORS: Record<string, { bg: string; border: string; dark: string }> = {
  function: { bg: "#dbeafe", border: "#3b82f6", dark: "#1e3a5f" },
  storage: { bg: "#fef3c7", border: "#f59e0b", dark: "#3d2e00" },
  external: { bg: "#f3e8ff", border: "#a855f7", dark: "#2e1a47" },
};

const SEVERITY_RING: Record<string, string> = {
  critical: "#ef4444",
  high: "#f97316",
  medium: "#f59e0b",
  low: "#6b7280",
};

/** Visual properties for each edge type. */
const EDGE_STYLE: Record<string, { color: string; dash?: string; label: string }> = {
  internal: { color: "#10b981", label: "Internal call" },
  calls: { color: "#a855f7", dash: "6 3", label: "External call" },
  mutates: { color: "#ef4444", label: "Mutates" },
  reads: { color: "#3b82f6", label: "Reads" },
};

interface LayoutNode extends CallGraphNode {
  x: number;
  y: number;
}

function layoutNodes(nodes: CallGraphNode[] = []): LayoutNode[] {
  if (!nodes || !Array.isArray(nodes)) {
    return [];
  }
  const functions = nodes.filter((n) => n.type === "function");
  const storages = nodes.filter((n) => n.type === "storage");
  const externals = nodes.filter((n) => n.type === "external");

  const laid: LayoutNode[] = [];
  const colSpacing = 280;
  const rowSpacing = 90;

  functions.forEach((n, i) => {
    laid.push({ ...n, x: 60, y: 60 + i * rowSpacing });
  });
  storages.forEach((n, i) => {
    laid.push({ ...n, x: 60 + colSpacing, y: 60 + i * rowSpacing });
  });
  externals.forEach((n, i) => {
    laid.push({ ...n, x: 60 + colSpacing * 2, y: 60 + i * rowSpacing });
  });

  return laid;
}

export const CallGraph = memo(function CallGraph({ nodes, edges }: CallGraphProps) {
  const [showLargeGraph, setShowLargeGraph] = useState(false);
  const layout = useMemo(() => layoutNodes(nodes), [nodes]);

  const nodeMap = useMemo(() => {
    const m = new Map<string, LayoutNode>();
    layout.forEach((n) => m.set(n.id, n));
    return m;
  }, [layout]);

  if (!nodes || nodes.length === 0) {
    return (
      <div className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-6">
        <h3 className="text-sm font-semibold text-zinc-700 dark:text-zinc-300 mb-4">
          Contract Interaction Graph
        </h3>
        <p className="text-sm text-zinc-500 dark:text-zinc-400 text-center py-8">
          No cross-contract call paths were reported for this scan.
        </p>
      </div>
    );
  }

  const isLarge = nodes && nodes.length > RENDER_THRESHOLD;
  const shouldRender = !isLarge || showLargeGraph;

  const maxX = Math.max(...layout.map((n) => n.x)) + 180;
  const maxY = Math.max(...layout.map((n) => n.y)) + 60;
  const svgWidth = Math.max(maxX, 500);
  const svgHeight = Math.max(maxY, 200);

  const nodeWidth = 140;
  const nodeHeight = 40;

  const internalCount = edges.filter((e) => e.type === "internal").length;
  const externalCount = edges.filter((e) => e.type === "calls").length;

  return (
    <div data-testid="call-graph-container" className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-6">
      <div className="flex flex-wrap justify-between items-start gap-2 mb-4">
        <div>
          <h3 className="text-sm font-semibold text-zinc-700 dark:text-zinc-300">
            Contract Interaction Graph
          </h3>
          <p className="text-xs text-zinc-500 dark:text-zinc-400 mt-0.5">
            {nodes.length} contract{nodes.length !== 1 ? "s" : ""}
            {" · "}
            <span className="text-emerald-600 dark:text-emerald-400">{internalCount} internal</span>
            {" · "}
            <span className="text-purple-600 dark:text-purple-400">{externalCount} external</span>
          </p>
        </div>
      </div>

      {!shouldRender ? (
        <div className="p-12 text-center border border-dashed border-zinc-300 dark:border-zinc-700 rounded-lg">
          <p className="text-sm text-zinc-500 mb-4">
            Large graph detected. Rendering many nodes may impact performance.
          </p>
          <button
            onClick={() => setShowLargeGraph(true)}
            className="rounded-lg bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 px-4 py-2 text-sm font-medium hover:bg-zinc-800 dark:hover:bg-zinc-200"
          >
            Show Graph Anyway
          </button>
        </div>
      ) : (
        <>
          {/* Legend */}
          <div className="flex flex-wrap gap-x-5 gap-y-2 mb-4 text-[10px] sm:text-xs text-zinc-500 dark:text-zinc-400">
            <span className="flex items-center gap-1.5 font-medium text-zinc-600 dark:text-zinc-300">Nodes:</span>
            {(["function", "storage", "external"] as const).map((type) => (
              <span key={type} className="flex items-center gap-1">
                <span
                  className="inline-block w-3 h-3 rounded"
                  style={{
                    background: NODE_COLORS[type].bg,
                    border: `2px solid ${NODE_COLORS[type].border}`,
                  }}
                />
                {type.charAt(0).toUpperCase() + type.slice(1)}
              </span>
            ))}
            <span className="flex items-center gap-1.5 font-medium text-zinc-600 dark:text-zinc-300 ml-2">Edges:</span>
            {(["internal", "calls", "mutates", "reads"] as const).map((type) => {
              const style = EDGE_STYLE[type];
              return (
                <span key={type} className="flex items-center gap-1">
                  <svg width="20" height="8" aria-hidden="true">
                    <line
                      x1="0" y1="4" x2="20" y2="4"
                      stroke={style.color}
                      strokeWidth={2}
                      strokeDasharray={style.dash}
                    />
                  </svg>
                  {style.label}
                </span>
              );
            })}
          </div>

          <div className="overflow-auto max-h-[600px]">
            <svg
              width={svgWidth}
              height={svgHeight}
              viewBox={`0 0 ${svgWidth} ${svgHeight}`}
              className="bg-zinc-50 dark:bg-zinc-950 rounded"
              role="img"
              aria-label="Contract interaction graph visualization"
            >
              <defs>
                {(["internal", "calls", "mutates", "reads"] as const).map((type) => (
                  <marker
                    key={type}
                    id={`arrowhead-${type}`}
                    markerWidth="8"
                    markerHeight="6"
                    refX="8"
                    refY="3"
                    orient="auto"
                  >
                    <polygon points="0 0, 8 3, 0 6" fill={EDGE_STYLE[type].color} />
                  </marker>
                ))}
              </defs>

              {edges.map((edge, i) => {
                const source = nodeMap.get(edge.source);
                const target = nodeMap.get(edge.target);
                if (!source || !target) return null;

                const x1 = source.x + nodeWidth;
                const y1 = source.y + nodeHeight / 2;
                const x2 = target.x;
                const y2 = target.y + nodeHeight / 2;
                const style = EDGE_STYLE[edge.type] ?? EDGE_STYLE.calls;

                // Curved path for internal edges to distinguish from straight external ones.
                const isCurved = edge.type === "internal";
                const midX = (x1 + x2) / 2;
                const midY = (y1 + y2) / 2 - 30;
                const d = isCurved
                  ? `M ${x1} ${y1} Q ${midX} ${midY} ${x2} ${y2}`
                  : undefined;

                return isCurved ? (
                  <path
                    key={`edge-${i}`}
                    d={d}
                    fill="none"
                    stroke={style.color}
                    strokeWidth={2}
                    strokeDasharray={style.dash}
                    markerEnd={`url(#arrowhead-${edge.type})`}
                  />
                ) : (
                  <line
                    key={`edge-${i}`}
                    x1={x1}
                    y1={y1}
                    x2={x2}
                    y2={y2}
                    stroke={style.color}
                    strokeWidth={2}
                    strokeDasharray={style.dash}
                    markerEnd={`url(#arrowhead-${edge.type})`}
                  />
                );
              })}

              {layout.map((node) => {
                const colors = NODE_COLORS[node.type] ?? NODE_COLORS.function;
                const severityColor = node.severity ? SEVERITY_RING[node.severity] : undefined;

                return (
                  <g key={node.id}>
                    {severityColor && (
                      <rect
                        x={node.x - 3}
                        y={node.y - 3}
                        width={nodeWidth + 6}
                        height={nodeHeight + 6}
                        rx={10}
                        fill="none"
                        stroke={severityColor}
                        strokeWidth={2}
                        strokeDasharray="4 2"
                      />
                    )}
                    <rect
                      x={node.x}
                      y={node.y}
                      width={nodeWidth}
                      height={nodeHeight}
                      rx={8}
                      fill={colors.bg}
                      stroke={colors.border}
                      strokeWidth={2}
                    />
                    <text
                      x={node.x + nodeWidth / 2}
                      y={node.y + nodeHeight / 2 + 4}
                      textAnchor="middle"
                      fontSize={11}
                      fontWeight={600}
                      fill="#1f2937"
                    >
                      {node.label.length > 16 ? node.label.slice(0, 14) + "…" : node.label}
                    </text>
                  </g>
                );
              })}
            </svg>
          </div>
        </>
      )}
    </div>
  );
});
