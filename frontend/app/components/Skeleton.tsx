import type { CSSProperties } from "react";

type Rounded = "none" | "sm" | "md" | "lg" | "xl" | "full";

const ROUNDED: Record<Rounded, string> = {
  none: "rounded-none",
  sm: "rounded-sm",
  md: "rounded-md",
  lg: "rounded-lg",
  xl: "rounded-xl",
  full: "rounded-full",
};

interface SkeletonProps {
  /** CSS width (e.g. `"60%"`, `120`). Numbers are pixels. Defaults to full width. */
  width?: string | number;
  /** CSS height (e.g. `"1rem"`, `24`). Numbers are pixels. */
  height?: string | number;
  rounded?: Rounded;
  className?: string;
}

/** A pulsing placeholder shape shown while content loads. */
export function Skeleton({ width, height = "1rem", rounded = "md", className = "" }: SkeletonProps) {
  const style: CSSProperties = { width, height };
  return (
    <div
      aria-hidden="true"
      data-testid="skeleton"
      style={style}
      className={`animate-pulse bg-zinc-200 dark:bg-zinc-800 theme-high-contrast:bg-zinc-700 ${ROUNDED[rounded]} ${className}`}
    />
  );
}

/** Placeholder for one table row of the findings list. */
export function FindingRowSkeleton() {
  return (
    <div className="flex items-center gap-4 border-b border-zinc-100 dark:border-zinc-800 px-4 py-3">
      <Skeleton width={64} height={20} rounded="full" />
      <div className="flex-1 space-y-2">
        <Skeleton width="55%" height={14} />
        <Skeleton width="30%" height={10} />
      </div>
      <Skeleton width={72} height={14} />
    </div>
  );
}

/** Placeholder rows shown while a findings list loads. */
export function FindingsListSkeleton({ rows = 5 }: { rows?: number }) {
  return (
    <div
      role="status"
      aria-busy="true"
      aria-label="Loading findings"
      className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900"
    >
      {Array.from({ length: rows }, (_, i) => (
        <FindingRowSkeleton key={i} />
      ))}
    </div>
  );
}

/** Placeholder for a contract card. */
export function ContractCardSkeleton() {
  return (
    <div className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4 space-y-3">
      <div className="flex items-center justify-between gap-3">
        <Skeleton width="45%" height={16} />
        <Skeleton width={56} height={20} rounded="full" />
      </div>
      <Skeleton width="80%" height={12} />
      <Skeleton width="35%" height={12} />
    </div>
  );
}

/** Placeholder for the dashboard's summary cards and chart. */
export function DashboardCardsSkeleton() {
  return (
    <section
      role="status"
      aria-busy="true"
      aria-label="Loading dashboard"
      className="grid grid-cols-1 md:grid-cols-2 gap-6"
    >
      {[0, 1].map((i) => (
        <div
          key={i}
          className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-6 space-y-4"
        >
          <Skeleton width="40%" height={18} />
          <Skeleton width="100%" height={i === 0 ? 96 : 140} rounded="lg" />
          <Skeleton width="65%" height={12} />
        </div>
      ))}
    </section>
  );
}
