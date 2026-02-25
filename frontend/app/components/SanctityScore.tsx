"use client";

import { useMemo } from "react";
import type { Finding } from "../types";

interface SanctityScoreProps {
  findings: Finding[];
}

const SEVERITY_WEIGHTS: Record<string, number> = {
  critical: 15,
  high: 10,
  medium: 5,
  low: 2,
};

function calculateScore(findings: Finding[]): number {
  let score = 100;
  for (const f of findings) {
    score -= SEVERITY_WEIGHTS[f.severity] ?? 0;
  }
  return Math.max(0, Math.min(100, score));
}

function getGrade(score: number): string {
  if (score >= 90) return "A";
  if (score >= 80) return "B";
  if (score >= 65) return "C";
  if (score >= 50) return "D";
  return "F";
}

function getColor(score: number): string {
  if (score >= 76) return "#22c55e";
  if (score >= 61) return "#f59e0b";
  if (score >= 41) return "#f97316";
  return "#ef4444";
}

export function SanctityScore({ findings }: SanctityScoreProps) {
  const score = useMemo(() => calculateScore(findings), [findings]);
  const grade = getGrade(score);
  const color = getColor(score);

  const radius = 70;
  const strokeWidth = 12;
  const circumference = Math.PI * radius;
  const progress = (score / 100) * circumference;

  return (
    <div className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-6">
      <h3 className="text-sm font-semibold text-zinc-700 dark:text-zinc-300 mb-4">
        Sanctity Score
      </h3>
      <div className="flex items-center justify-center">
        <svg width="180" height="110" viewBox="0 0 180 110">
          {/* Background arc */}
          <path
            d={`M ${90 - radius} 95 A ${radius} ${radius} 0 0 1 ${90 + radius} 95`}
            fill="none"
            stroke="currentColor"
            strokeWidth={strokeWidth}
            className="text-zinc-200 dark:text-zinc-700"
            strokeLinecap="round"
          />
          {/* Progress arc */}
          <path
            d={`M ${90 - radius} 95 A ${radius} ${radius} 0 0 1 ${90 + radius} 95`}
            fill="none"
            stroke={color}
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeDasharray={`${progress} ${circumference}`}
          />
          {/* Score text */}
          <text
            x="90"
            y="75"
            textAnchor="middle"
            className="fill-zinc-900 dark:fill-zinc-100"
            fontSize="28"
            fontWeight="bold"
          >
            {score}
          </text>
          {/* Grade label */}
          <text
            x="90"
            y="95"
            textAnchor="middle"
            fontSize="14"
            fontWeight="600"
            fill={color}
          >
            Grade: {grade}
          </text>
        </svg>
      </div>
      <p className="text-center text-xs text-zinc-500 dark:text-zinc-400 mt-2">
        {score >= 76
          ? "Good security posture"
          : score >= 50
            ? "Moderate risk — review findings"
            : "High risk — immediate attention needed"}
      </p>
    </div>
  );
}

export { calculateScore };
