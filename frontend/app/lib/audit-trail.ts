/** Structured audit record written for every scan request. */
export interface ScanAuditEntry {
  timestamp: string;
  clientIp: string;
  fileName: string;
  fileSizeBytes: number;
  latencyMs: number;
  outcomeCode: number;
  totalFindings: number;
  hasCritical: boolean;
  hasHigh: boolean;
}

// Ring buffer — keeps the last N entries in-process (test visibility + light monitoring)
const MAX_LOG_SIZE = 1_000;
const _log: ScanAuditEntry[] = [];

/**
 * Appends an entry to the in-process audit ring buffer and emits a structured
 * log line for external aggregators (Datadog, CloudWatch, etc.).
 */
export function recordScanAudit(entry: ScanAuditEntry): void {
  if (_log.length >= MAX_LOG_SIZE) _log.shift();
  _log.push(entry);
  // Single-line JSON so log parsers see one event per line.
  console.log(JSON.stringify({ event: "scan_audit", ...entry }));
}

/** Returns a read-only snapshot of the current in-process audit log. */
export function getAuditLog(): readonly ScanAuditEntry[] {
  return _log;
}

/** Clears the in-process ring buffer (used in tests). */
export function clearAuditLog(): void {
  _log.length = 0;
}
