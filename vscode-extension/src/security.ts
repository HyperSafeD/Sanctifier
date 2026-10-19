/**
 * Security guards for the Sanctifier VS Code extension.
 * All functions are pure — no VS Code API dependency.
 */

import * as path from 'path';

/** Maximum size in bytes for a SARIF file the extension will process. */
export const MAX_SARIF_BYTES = 5 * 1024 * 1024; // 5 MB

/** Maximum number of results in a single SARIF run the extension will render. */
export const MAX_SARIF_RESULTS = 10_000;

/** Maximum source file size analysed in-editor (10 MB). */
export const MAX_SOURCE_BYTES = 10 * 1024 * 1024;

export interface ValidationResult {
  ok: boolean;
  error?: string;
}

/**
 * Validates that raw SARIF content is within safe processing bounds.
 */
export function validateSarifContent(raw: string): ValidationResult {
  const byteLen = Buffer.byteLength(raw, 'utf8');
  if (byteLen > MAX_SARIF_BYTES) {
    return { ok: false, error: `SARIF file is too large (${byteLen} bytes; limit ${MAX_SARIF_BYTES})` };
  }

  // Guard against degenerate inputs before JSON.parse
  if (raw.trim().length === 0) {
    return { ok: false, error: 'SARIF content is empty' };
  }

  return { ok: true };
}

/**
 * Counts total results across all runs in a parsed SARIF object and checks
 * they are within safe rendering limits.
 */
export function validateSarifResultCount(runs: Array<{ results?: unknown[] }>): ValidationResult {
  let total = 0;
  for (const run of runs) {
    total += run.results?.length ?? 0;
  }
  if (total > MAX_SARIF_RESULTS) {
    return {
      ok: false,
      error: `SARIF log contains ${total} results; limit is ${MAX_SARIF_RESULTS}`,
    };
  }
  return { ok: true };
}

/**
 * Checks that a workspace-relative file path does not escape the workspace root
 * (path traversal guard).
 */
export function isPathWithinWorkspace(workspaceRoot: string, filePath: string): boolean {
  const resolved = path.resolve(filePath);
  const root = path.resolve(workspaceRoot);
  return resolved.startsWith(root + path.sep) || resolved === root;
}

/**
 * Sanitises a string to be safe for use as a filename (strips path separators
 * and other shell-significant characters).
 */
export function sanitiseFilename(name: string): string {
  return name.replace(/[/\\:*?"<>|]/g, '_').slice(0, 255);
}
