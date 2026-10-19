"use client";

import { useMemo } from "react";
import type { ScanRecord } from "../lib/scan-history";

interface TrendChartProps {
  records: ScanRecord[];
  severity: "critical" | "high" | "medium" | "low";
  color: string;
  onClear?: () => void;
}

const CHART_HEIGHT = 80;
const CHART_WIDTH = 280;
const POINT_RADIUS = 3;

export function TrendChart({ records, severity, color, onClear }: TrendChartProps) {
  const pathData = useMemo(() => {
    if (records.length < 2) return null;

    const values = records.map((r) => {
      switch (severity) {
        case "critical": return r.critical;
        case "high": return r.high;
        case "medium": return r.medium;
        case "low": return r.low;
      }
    });

    const maxVal = Math.max(...values, 1);
    const padding = 4;
    const graphHeight = CHART_HEIGHT - padding * 2;
    const graphWidth = CHART_WIDTH - padding * 2;

    const points = values.map((v, i) => {
      const x = padding + (i / Math.max(values.length - 1, 1)) * graphWidth;
      const y = padding + graphHeight - (v / maxVal) * graphHeight;
      return `${x},${y}`;
    });

    return {
      points: points.map((p) => p.split(",").map(Number)),
      line: `M${points.join(" L")}`,
      area: `M${points[0]} L${points.join(" L")} L${points[points.length - 1].split(",")[0]},${CHART_HEIGHT - padding} Z`,
      maxVal,
    };
  }, [records, severity]);

  if (records.length === 0) {
    return (
      <div className="flex items-center justify-center h-20 text-xs text-zinc-500">
        No data yet
      </div>
    );
  }

  const label =
    severity.charAt(0).toUpperCase() + severity.slice(1);

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between">
        <span className="text-[10px] font-medium uppercase tracking-wider text-zinc-400">
          {label}
        </span>
        <span className="text-[10px] text-zinc-500">
          {records.length} scan{records.length !== 1 ? "s" : ""}
        </span>
      </div>
      <svg
        viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
        className="w-full overflow-visible"
        aria-label={`${label} severity trend over ${records.length} scans`}
        role="img"
      >
        {pathData && (
          <>
            {/* Area fill */}
            <path
              d={pathData.area}
              fill={color}
              fillOpacity={0.1}
            />
            {/* Line */}
            <path
              d={pathData.line}
              fill="none"
              stroke={color}
              strokeWidth={1.5}
              strokeLinejoin="round"
              strokeLinecap="round"
            />
            {/* Points */}
            {pathData.points.map(([x, y], i) => (
              <circle
                key={i}
                cx={x}
                cy={y}
                r={POINT_RADIUS}
                fill={color}
                className="hover:r-[5] transition-all"
              >
                <title>
                  {new Date(records[i].timestamp).toLocaleDateString()}:{" "}
                  {severity === "critical"
                    ? records[i].critical
                    : severity === "high"
                      ? records[i].high
                      : severity === "medium"
                        ? records[i].medium
                        : records[i].low}{" "}
                  findings
                </title>
              </circle>
            ))}
          </>
        )}
      </svg>
      {onClear && records.length > 0 && (
        <button
          onClick={onClear}
          className="text-[10px] text-zinc-500 hover:text-zinc-300 transition-colors"
          aria-label={`Clear ${label} history`}
        >
          Clear history
        </button>
      )}
    </div>
  );
}
