export { CODES, SEVERITY_ORDER, type Severity, type EditorFinding, filterBySeverity } from './analyzer';

/**
 * Stable public API vended by the extension's activate() return value.
 * Other extensions can consume it via:
 *   const api = vscode.extensions.getExtension<SanctifierExtensionApi>('sanctifier.sanctifier-soroban')?.exports;
 */
export interface SanctifierExtensionApi {
  /** Extension version from package.json. */
  readonly version: string;
  /**
   * Returns the current in-editor findings for a document URI string.
   * Returns an empty array if the document is not tracked or the extension is disabled.
   */
  getFindings(documentUri: string): import('./analyzer').EditorFinding[];
}
