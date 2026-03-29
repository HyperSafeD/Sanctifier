"use client";

import { useMemo } from "react";
import type { Finding } from "../types";
import {
  calculateScore,
  getScoreGrade,
  getScoreNarrative,
  getSeverityColor,
} from "../lib/report-export";

interface SanctityScoreProps {
  findings: Finding[];
}

export function SanctityScore({ findings }: SanctityScoreProps) {
  const score = useMemo(() => calculateScore(findings), [findings]);
  const grade = getScoreGrade(score);
  const color = getSeverityColor(score);
  const narrative = getScoreNarrative(score);

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
        <svg
          viewBox="0 0 180 110"
          className="w-full h-auto max-w-[180px]"
          role="img"
          aria-label={`Sanctity score: ${score} out of 100. Grade: ${grade}. ${narrative}`}
        >
          <title>Sanctity Score: {score}/100, Grade {grade}</title>
          <path
            d={`M ${90 - radius} 95 A ${radius} ${radius} 0 0 1 ${90 + radius} 95`}
            fill="none"
            stroke="currentColor"
            strokeWidth={strokeWidth}
            className="text-zinc-200 dark:text-zinc-700"
            strokeLinecap="round"
          />
          <path
            d={`M ${90 - radius} 95 A ${radius} ${radius} 0 0 1 ${90 + radius} 95`}
            fill="none"
            stroke={color}
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeDasharray={`${progress} ${circumference}`}
          />
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
        {narrative}
      </p>
    </div>
  );
}

export { calculateScore };
