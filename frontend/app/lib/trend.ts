import type { ScanRecord } from "./scan-history";

export const TREND_WINDOW_DAYS = 30;
const DAY_MS = 24 * 60 * 60 * 1000;

/** Scan records from the last `days` days, oldest first. */
export function recentScanRecords(
  records: ScanRecord[],
  days = TREND_WINDOW_DAYS,
  now = Date.now(),
): ScanRecord[] {
  const cutoff = now - days * DAY_MS;
  return records.filter((r) => r.timestamp >= cutoff).sort((a, b) => a.timestamp - b.timestamp);
}
