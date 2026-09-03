/**
 * Utilities for formatting messages written to the Sanctifier output channel.
 *
 * Centralising the format here keeps the strings testable and consistent
 * across every call site in extension.ts.
 */

export const PREFIX = '[sanctifier]';

export type OutputLine =
  | { kind: 'info';    text: string }
  | { kind: 'stderr';  text: string }
  | { kind: 'error';   text: string }
  | { kind: 'debug';   text: string }
  | { kind: 'result';  json: string }
  | { kind: 'done' };

export function formatLine(line: OutputLine): string {
  switch (line.kind) {
    case 'info':
      return `${PREFIX} ${line.text}`;
    case 'stderr':
      return `${PREFIX}[stderr] ${line.text}`;
    case 'error':
      return `${PREFIX}[error] ${line.text}`;
    case 'debug':
      return `${PREFIX}[debug] ${line.text}`;
    case 'result':
      return line.json;
    case 'done':
      return `${PREFIX} Scan complete.`;
  }
}

export function formatScanStart(fsPath: string): string {
  return formatLine({ kind: 'info', text: `Scanning ${fsPath} …` });
}

export function formatCliExit(code: number): string {
  return formatLine({ kind: 'info', text: `CLI exited with code ${code}.` });
}

export function formatFindings(count: number): string {
  if (count === 0) return formatLine({ kind: 'info', text: 'No findings.' });
  return formatLine({
    kind: 'info',
    text: `${count} finding${count === 1 ? '' : 's'} reported.`,
  });
}

/**
 * Render a batch of output lines as a single string ready to dump to the
 * output channel (each line separated by \n).
 */
export function renderOutputBlock(lines: OutputLine[]): string {
  return lines.map(formatLine).join('\n');
}

/**
 * Truncate long output to avoid flooding the output panel.
 * Returns the original string if it is within the budget, or a truncated
 * version with a summary suffix.
 */
export function truncateOutput(
  text: string,
  maxBytes = 256 * 1024,
): string {
  const buf = Buffer.byteLength(text, 'utf8');
  if (buf <= maxBytes) return text;
  const ratio = maxBytes / buf;
  const cutAt = Math.floor(text.length * ratio);
  const truncated = text.slice(0, cutAt);
  const dropped = text.length - cutAt;
  return `${truncated}\n${PREFIX} [output truncated — ${dropped} characters omitted]`;
}
