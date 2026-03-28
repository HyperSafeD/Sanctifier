"use client";

import { useMemo, useState, memo } from "react";
import type { CallGraphNode, CallGraphEdge } from "../types";

interface CallGraphProps {
  nodes: CallGraphNode[];
  edges: CallGraphEdge[];
}

const RENDER_THRESHOLD = 100;

const NODE_COLORS: Record<string, { bg: string; border: string }> = {
  function: { bg: "#dbeafe", border: "#3b82f6" },
  storage: { bg: "#fef3c7", border: "#f59e0b" },
  external: { bg: "#f3e8ff", border: "#a855f7" },
};

const SEVERITY_RING: Record<string, string> = {
  critical: "#ef4444",
  high: "#f97316",
  medium: "#f59e0b",
  low: "#6b7280",
};

const EDGE_COLORS: Record<string, string> = {
  calls: "#6b7280",
  mutates: "#ef4444",
  reads: "#3b82f6",
};

interface LayoutNode extends CallGraphNode {
  x: number;
  y: number;
}

function layoutNodes(nodes: CallGraphNode[]): LayoutNode[] {
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

  if (nodes.length === 0) {
    return (
      <div className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-6">
        <h3 className="text-sm font-semibold text-zinc-700 dark:text-zinc-300 mb-4">
          Contract Call Graph
        </h3>
        <p className="text-sm text-zinc-500 dark:text-zinc-400 text-center py-8">
          No cross-contract call paths were reported for this scan.
        </p>
      </div>
    );
  }

  const isLarge = nodes.length > RENDER_THRESHOLD;
  const shouldRender = !isLarge || showLargeGraph;

  const maxX = Math.max(...layout.map((n) => n.x)) + 180;
  const maxY = Math.max(...layout.map((n) => n.y)) + 60;
  const svgWidth = Math.max(maxX, 500);
  const svgHeight = Math.max(maxY, 200);

  const nodeWidth = 140;
  const nodeHeight = 40;

  return (
    <div className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-6">
      <div className="flex justify-between items-center mb-4">
        <h3 className="text-sm font-semibold text-zinc-700 dark:text-zinc-300">
          Contract Call Graph ({nodes.length} nodes)
        </h3>
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
          <div className="flex flex-wrap gap-x-4 gap-y-2 mb-4 text-[10px] sm:text-xs text-zinc-500 dark:text-zinc-400">
            <span className="flex items-center gap-1">
              <span className="inline-block w-3 h-3 rounded" style={{ background: NODE_COLORS.function.bg, border: `2px solid ${NODE_COLORS.function.border}` }} />
              Function
            </span>
            <span className="flex items-center gap-1">
              <span className="inline-block w-3 h-3 rounded" style={{ background: NODE_COLORS.storage.bg, border: `2px solid ${NODE_COLORS.storage.border}` }} />
              Storage
            </span>
            <span className="flex items-center gap-1">
              <span className="inline-block w-2 h-0.5" style={{ background: EDGE_COLORS.mutates }} />
              Mutates
            </span>
            <span className="flex items-center gap-1">
              <span className="inline-block w-2 h-0.5" style={{ background: EDGE_COLORS.calls }} />
              Calls
            </span>
          </div>
          <div className="overflow-auto max-h-[600px]">
            <svg
              width={svgWidth}
              height={svgHeight}
              viewBox={`0 0 ${svgWidth} ${svgHeight}`}
              className="bg-zinc-50 dark:bg-zinc-950 rounded"
              role="img"
              aria-label="Contract call graph visualization"
            >
              <defs>
                <marker id="arrowhead-mutates" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
                  <polygon points="0 0, 8 3, 0 6" fill={EDGE_COLORS.mutates} />
                </marker>
                <marker id="arrowhead-calls" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
                  <polygon points="0 0, 8 3, 0 6" fill={EDGE_COLORS.calls} />
                </marker>
                <marker id="arrowhead-reads" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
                  <polygon points="0 0, 8 3, 0 6" fill={EDGE_COLORS.reads} />
                </marker>
              </defs>

              {edges.map((edge, i) => {
                const source = nodeMap.get(edge.source);
                const target = nodeMap.get(edge.target);
                if (!source || !target) return null;

                const x1 = source.x + nodeWidth;
                const y1 = source.y + nodeHeight / 2;
                const x2 = target.x;
                const y2 = target.y + nodeHeight / 2;
                const color = EDGE_COLORS[edge.type] || EDGE_COLORS.calls;

                return (
                  <line
                    key={`edge-${i}`}
                    x1={x1}
                    y1={y1}
                    x2={x2}
                    y2={y2}
                    stroke={color}
                    strokeWidth={2}
                    markerEnd={`url(#arrowhead-${edge.type})`}
                  />
                );
              })}

              {layout.map((node) => {
                const colors = NODE_COLORS[node.type] || NODE_COLORS.function;
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
                      {node.label.length > 16 ? node.label.slice(0, 14) + "..." : node.label}
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
