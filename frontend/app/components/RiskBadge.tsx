import type { RiskLevel } from "../lib/contracts-model";

const STYLES: Record<RiskLevel, string> = {
  critical: "bg-red-100 text-red-800 dark:bg-red-950 dark:text-red-300 theme-high-contrast:bg-red-700 theme-high-contrast:text-white",
  high: "bg-orange-100 text-orange-800 dark:bg-orange-950 dark:text-orange-300 theme-high-contrast:bg-orange-600 theme-high-contrast:text-black",
  medium: "bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-300 theme-high-contrast:bg-yellow-300 theme-high-contrast:text-black",
  low: "bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300 theme-high-contrast:bg-green-400 theme-high-contrast:text-black",
  info: "bg-zinc-100 text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300 theme-high-contrast:bg-white theme-high-contrast:text-black",
};

export function RiskBadge({ risk }: { risk: RiskLevel }) {
  return (
    <span
      className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold capitalize ${STYLES[risk]}`}
    >
      {risk}
    </span>
  );
}
