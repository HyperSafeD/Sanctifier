export const SCAN_PROGRESS_PHASES = [
  "Parsing Soroban SDK attributes...",
  "Building intermediate representation...",
  "Traversing call graph...",
  "Running static analysis rules...",
  "Checking for authorization gaps...",
  "Verifying arithmetic safety...",
  "Estimating ledger footprint...",
] as const;

export function nextScanProgressPhase(index: number): string {
  return SCAN_PROGRESS_PHASES[index % SCAN_PROGRESS_PHASES.length];
}
