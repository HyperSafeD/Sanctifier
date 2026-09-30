/**
 * Skeleton placeholder for `CallGraph` (issue #1446).
 *
 * Deliberately kept in its own tiny module, separate from `CallGraph.tsx`:
 * this component is meant to be imported *statically* as the `loading:`
 * fallback for `next/dynamic(() => import("./CallGraph"))`, and a static
 * import from `CallGraph.tsx` itself would drag the whole (SVG-heavy)
 * call-graph renderer into the eagerly-loaded bundle, defeating the point of
 * code-splitting it in the first place.
 *
 * Mirrors the real component's layout — title, stats line, legend row, graph
 * area — instead of a bare "Loading…" string, so the page doesn't visibly
 * reflow once the real content mounts.
 */
export function CallGraphSkeleton() {
  return (
    <div
      data-testid="call-graph-skeleton"
      className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-6 animate-pulse"
      role="status"
      aria-label="Loading contract interaction graph"
    >
      <div className="flex flex-wrap justify-between items-start gap-2 mb-4">
        <div className="w-full max-w-xs">
          <div className="h-4 w-48 rounded bg-zinc-200 dark:bg-zinc-700 mb-2" />
          <div className="h-3 w-64 rounded bg-zinc-100 dark:bg-zinc-800" />
        </div>
      </div>
      <div className="flex flex-wrap gap-3 mb-4">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="h-3 w-16 rounded bg-zinc-100 dark:bg-zinc-800" />
        ))}
      </div>
      <div className="h-[220px] rounded bg-zinc-100 dark:bg-zinc-800" />
    </div>
  );
}
