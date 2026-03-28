/**
 * Lightweight in-editor checks aligned with sanctifier-core finding codes.
 * Line numbers are 1-based (VS Code Document convention).
 */

export const CODES = {
  AUTH_GAP: 'S001',
  PANIC_USAGE: 'S002',
  ARITHMETIC_OVERFLOW: 'S003',
  UNSAFE_PATTERN: 'S006',
} as const;

export type Severity = 'error' | 'warning' | 'information';

export interface EditorFinding {
  line: number;
  code: string;
  severity: Severity;
  message: string;
  endLine?: number;
  endCharacter?: number;
}

const PUB_FN = /^\s*pub\s+fn\s+(\w+)\s*\(/;

function lineHasRequireAuth(line: string): boolean {
  const code = line.replace(/\/\/.*$/, '').trim();
  return (
    code.includes('require_auth') ||
    code.includes('require_auth_for_args') ||
    code.includes('.require_auth()')
  );
}

/** Heuristic across a whole function: Soroban storage mutation or cross-contract call. */
function bodySuggestsPrivilegedOp(body: string): boolean {
  const b = body.replace(/\/\/[^\n]*/g, ' ').replace(/\s+/g, ' ');
  if (b.includes('invoke_contract')) {
    return true;
  }
  const touchesLedgerBucket =
    b.includes('.persistent()') || b.includes('.temporary()') || b.includes('.instance()');
  const mutates =
    b.includes('.set(') || b.includes('.remove(') || b.includes('.update(');
  return touchesLedgerBucket && mutates && b.includes('storage()');
}

function contractImplRegions(lines: string[]): { start: number; end: number }[] {
  const regions: { start: number; end: number }[] = [];
  let i = 0;
  while (i < lines.length) {
    if (!/^\s*#\s*\[\s*contractimpl\b/.test(lines[i])) {
      i++;
      continue;
    }
    // Find `impl ... {`
    let j = i + 1;
    let implLine = -1;
    while (j < lines.length && j < i + 40) {
      if (/^\s*impl\b/.test(lines[j])) {
        implLine = j;
        break;
      }
      j++;
    }
    if (implLine < 0) {
      i++;
      continue;
    }
    let k = implLine;
    let depth = 0;
    let opened = false;
    for (; k < lines.length; k++) {
      for (const c of lines[k]) {
        if (c === '{') {
          depth++;
          opened = true;
        } else if (c === '}') {
          depth--;
        }
      }
      if (opened && depth === 0) {
        regions.push({ start: implLine, end: k });
        i = k + 1;
        break;
      }
    }
    if (!opened || depth !== 0) {
      i = implLine + 1;
    }
  }
  return regions;
}

function extractFunctionBlock(lines: string[], pubFnLineIdx: number): string[] | null {
  let depth = 0;
  let started = false;
  const chunk: string[] = [];
  for (let k = pubFnLineIdx; k < lines.length; k++) {
    chunk.push(lines[k]);
    for (const c of lines[k]) {
      if (c === '{') {
        depth++;
        started = true;
      } else if (c === '}') {
        depth--;
      }
    }
    if (started && depth === 0) {
      return chunk;
    }
  }
  return null;
}

function analyzeAuthGaps(lines: string[], findings: EditorFinding[]): void {
  const regions = contractImplRegions(lines);
  for (const { start, end } of regions) {
    const slice = lines.slice(start, end + 1);
    for (let li = 0; li < slice.length; li++) {
      const line = slice[li];
      const m = line.match(PUB_FN);
      if (!m) {
        continue;
      }
      const absLine = start + li + 1;
      const block = extractFunctionBlock(slice, li);
      if (!block) {
        continue;
      }
      const body = block.join('\n');
      const hasPrivileged = bodySuggestsPrivilegedOp(body);
      const hasAuth = body.split('\n').some(lineHasRequireAuth);
      if (hasPrivileged && !hasAuth) {
        findings.push({
          line: absLine,
          code: CODES.AUTH_GAP,
          severity: 'warning',
          message: `Function '${m[1]}' may perform a privileged operation without authentication (require_auth / require_auth_for_args).`,
        });
      }
    }
  }
}

function analyzeUnsafePatterns(lines: string[], findings: EditorFinding[]): void {
  const unwrapOrExpect = /\.unwrap\s*\(\s*\)|\.expect\s*\(/;
  const panic = /\bpanic!\s*\(/;
  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const codePart = raw.split('//')[0];
    if (unwrapOrExpect.test(codePart)) {
      const isExpect = /\.expect\s*\(/.test(codePart);
      findings.push({
        line: i + 1,
        code: isExpect ? CODES.UNSAFE_PATTERN : CODES.UNSAFE_PATTERN,
        severity: 'warning',
        message: isExpect
          ? '.expect() can abort the contract if the precondition fails. Prefer explicit error handling.'
          : '.unwrap() can abort the contract. Use ?, match, or typed errors.',
      });
    }
    if (panic.test(codePart)) {
      findings.push({
        line: i + 1,
        code: CODES.PANIC_USAGE,
        severity: 'error',
        message: 'panic! aborts the contract. Return Result or use structured errors.',
      });
    }
  }
}

/** Conservative: identifier-like operands with +, -, or * (inside contractimpl only). */
function analyzeArithmeticHeuristic(lines: string[], findings: EditorFinding[]): void {
  const inContract = new Set<number>();
  for (const { start, end } of contractImplRegions(lines)) {
    for (let L = start; L <= end; L++) {
      inContract.add(L);
    }
  }
  const opBetweenIds = /\b[a-zA-Z_]\w*\s*(\+|\-|\*)\s*[a-zA-Z_]\w*\b/;
  for (let i = 0; i < lines.length; i++) {
    if (!inContract.has(i)) {
      continue;
    }
    const raw = lines[i];
    const codePart = raw.split('//')[0];
    if (
      !opBetweenIds.test(codePart) ||
      codePart.includes('checked_') ||
      codePart.includes('saturating_') ||
      codePart.includes('wrapping_')
    ) {
      continue;
    }
    findings.push({
      line: i + 1,
      code: CODES.ARITHMETIC_OVERFLOW,
      severity: 'warning',
      message:
        'Unchecked arithmetic may overflow/underflow. Consider checked_add/sub/mul or Soroban i128 patterns.',
    });
  }
}

export function looksLikeSorobanSource(text: string): boolean {
  return (
    /#\s*\[\s*contractimpl\b/.test(text) ||
    /\bsoroban_sdk\b/.test(text) ||
    /\#\[contract\b/.test(text) ||
    /\bcontractimpl\b/.test(text)
  );
}

/**
 * @param text Full document text (current buffer — works while typing).
 */
export function analyzeSorobanSource(text: string): EditorFinding[] {
  const lines = text.split(/\r?\n/);
  const findings: EditorFinding[] = [];
  analyzeUnsafePatterns(lines, findings);
  analyzeAuthGaps(lines, findings);
  analyzeArithmeticHeuristic(lines, findings);

  // De-duplicate same line + code
  const key = (f: EditorFinding) => `${f.line}:${f.code}:${f.message.slice(0, 40)}`;
  const seen = new Set<string>();
  return findings.filter((f) => {
    const k = key(f);
    if (seen.has(k)) {
      return false;
    }
    seen.add(k);
    return true;
  });
}
