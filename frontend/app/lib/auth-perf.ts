/**
 * Performance budgets for the access-control / auth integration path.
 *
 * These constants are the single source of truth referenced by both unit tests
 * and CI performance gates. Tighten them as the implementation matures.
 */
export const PERF_BUDGETS = {
  /** Rate-limit map lookup + update (ms). */
  RATE_LIMIT_CHECK_MS: 5,
  /** MIME-type + size validation for an upload (ms). */
  UPLOAD_VALIDATION_MS: 10,
  /** Full analyze API round-trip p95 budget (ms). */
  ANALYZE_REQUEST_P95_MS: 30_000,
  /** Individual rate-limit map lookup (ms). */
  RATE_LIMIT_LOOKUP_MS: 2,
} as const;

export type PerfBudgetKey = keyof typeof PERF_BUDGETS;

export interface PerfMeasurement {
  label: PerfBudgetKey;
  durationMs: number;
  withinBudget: boolean;
  budget: number;
}

/** Wraps a duration and reports whether it falls inside the declared budget. */
export function measureAgainstBudget(
  label: PerfBudgetKey,
  durationMs: number
): PerfMeasurement {
  const budget = PERF_BUDGETS[label];
  return { label, durationMs, withinBudget: durationMs <= budget, budget };
}

/**
 * Times the execution of `fn` and returns a PerfMeasurement.
 * Useful for wrapping access-control functions in unit tests.
 */
export async function timeAsync<T>(
  label: PerfBudgetKey,
  fn: () => Promise<T>
): Promise<{ result: T; measurement: PerfMeasurement }> {
  const start = performance.now();
  const result = await fn();
  const durationMs = performance.now() - start;
  return { result, measurement: measureAgainstBudget(label, durationMs) };
}
