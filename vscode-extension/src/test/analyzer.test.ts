import * as fs from 'fs';
import * as path from 'path';
import {
  analyzeSorobanSource,
  looksLikeSorobanSource,
  filterBySeverity,
  buildRequireAuthInsertLine,
  buildUnwrapFix,
  buildCheckedArithFix,
  CODES,
  SEVERITY_ORDER,
  type EditorFinding,
} from '../analyzer';

const fixturesDir = path.join(__dirname, '..', '..', 'src', 'test', 'fixtures');

function fixture(name: string): string {
  return fs.readFileSync(path.join(fixturesDir, name), 'utf8');
}

// Helper for inline tests (avoids fixture file proliferation for simple cases)
const SOROBAN_HEADER = `
#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Address};
`;

function wrap(body: string): string {
  return `${SOROBAN_HEADER}
pub struct Contract;

#[contractimpl]
impl Contract {
${body}
}
`;
}

// ---------------------------------------------------------------------------
// looksLikeSorobanSource
// ---------------------------------------------------------------------------

describe('looksLikeSorobanSource', () => {
  it('returns true for soroban_sdk import', () => {
    expect(looksLikeSorobanSource('use soroban_sdk::Env;')).toBe(true);
  });

  it('returns true for #[contractimpl]', () => {
    expect(looksLikeSorobanSource('#[contractimpl]\nimpl MyContract {}')).toBe(true);
  });

  it('returns true for #[contract]', () => {
    expect(looksLikeSorobanSource('#[contract]\npub struct Foo;')).toBe(true);
  });

  it('returns true for contractimpl keyword', () => {
    expect(looksLikeSorobanSource('contractimpl')).toBe(true);
  });

  it('returns false for plain Rust', () => {
    expect(looksLikeSorobanSource('fn main() { println!("hello"); }')).toBe(false);
  });

  it('returns false for empty string', () => {
    expect(looksLikeSorobanSource('')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// S001 – AUTH_GAP
// ---------------------------------------------------------------------------

const AUTH_GAP_SRC = wrap(`  pub fn privileged(env: Env, _caller: Address) {
    env.storage().persistent().set(&"key", &42u32);
  }`);

const AUTH_OK_SRC = wrap(`  pub fn guarded(env: Env, caller: Address) {
    caller.require_auth();
    env.storage().persistent().set(&"key", &42u32);
  }`);

const AUTH_FOR_ARGS_SRC = wrap(`  pub fn transfer(env: Env, from: Address, _to: Address, _amount: i128) {
    from.require_auth_for_args(());
    env.storage().persistent().set(&"key", &42u32);
  }`);

describe('S001 AUTH_GAP', () => {
  it('flags a pub fn that mutates storage without require_auth (inline)', () => {
    const findings = analyzeSorobanSource(AUTH_GAP_SRC);
    expect(findings.find((f) => f.code === CODES.AUTH_GAP)).toBeDefined();
  });

  it('does NOT flag a pub fn that calls require_auth before mutating storage (inline)', () => {
    const findings = analyzeSorobanSource(AUTH_OK_SRC);
    expect(findings.filter((f) => f.code === CODES.AUTH_GAP)).toHaveLength(0);
  });

  it('does not flag when require_auth_for_args is present (inline)', () => {
    const gaps = analyzeSorobanSource(AUTH_FOR_ARGS_SRC).filter((f) => f.code === CODES.AUTH_GAP);
    expect(gaps).toHaveLength(0);
  });

  it('flags a privileged fn missing require_auth (fixture)', () => {
    const src = fixture('auth_gap.rs');
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.AUTH_GAP)).toBe(true);
  });

  it('does not flag when require_auth is present (fixture)', () => {
    const src = fixture('safe_contract.rs');
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.AUTH_GAP)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// S002 – PANIC_USAGE
// ---------------------------------------------------------------------------

describe('S002 PANIC_USAGE', () => {
  it('flags panic! inside a contract (inline)', () => {
    const src = wrap('  pub fn blow_up(env: Env) {\n    panic!("never");\n  }');
    const findings = analyzeSorobanSource(src);
    const hit = findings.find((f) => f.code === CODES.PANIC_USAGE);
    expect(hit).toBeDefined();
    expect(hit?.severity).toBe('error');
  });

  it('does NOT flag panic! in a line comment (inline)', () => {
    const src = wrap('  pub fn ok(_env: Env) -> u32 {\n    // panic! is bad\n    42\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.filter((f) => f.code === CODES.PANIC_USAGE)).toHaveLength(0);
  });

  it('flags panic! with S002 (fixture)', () => {
    const src = fixture('panic_contract.rs');
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.PANIC_USAGE)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// S006 – UNSAFE_PATTERN (.unwrap / .expect)
// ---------------------------------------------------------------------------

describe('S006 UNSAFE_PATTERN', () => {
  it('flags .unwrap() inside a contract function (inline)', () => {
    const src = wrap('  pub fn risky(_env: Env) -> u32 {\n    let val: Option<u32> = Some(1);\n    val.unwrap()\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.find((f) => f.code === CODES.UNSAFE_PATTERN)).toBeDefined();
  });

  it('flags .expect("…") inside a contract function (inline)', () => {
    const src = wrap('  pub fn risky(_env: Env) -> u32 {\n    let val: Option<u32> = None;\n    val.expect("must exist")\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.find((f) => f.code === CODES.UNSAFE_PATTERN)).toBeDefined();
  });

  it('does NOT flag .unwrap() inside a line comment (inline)', () => {
    const src = wrap('  pub fn safe(_env: Env) -> u32 {\n    // val.unwrap() is bad\n    42\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.filter((f) => f.code === CODES.UNSAFE_PATTERN)).toHaveLength(0);
  });

  it('flags .unwrap() with S006 (fixture)', () => {
    const src = fixture('panic_contract.rs');
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.UNSAFE_PATTERN)).toBe(true);
  });

  it('does not flag commented-out unwrap (fixture)', () => {
    const src = '// let x = val.unwrap();\nfn safe() {}';
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.UNSAFE_PATTERN)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// S003 – ARITHMETIC_OVERFLOW
// ---------------------------------------------------------------------------

describe('S003 ARITHMETIC_OVERFLOW', () => {
  it('flags unchecked + between identifiers inside contractimpl (inline)', () => {
    const src = wrap('  pub fn add(_env: Env, a: u32, b: u32) -> u32 {\n    a + b\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.find((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toBeDefined();
  });

  it('does NOT flag checked_add (inline)', () => {
    const src = wrap('  pub fn safe_add(_env: Env, a: u32, b: u32) -> u32 {\n    a.checked_add(b).unwrap_or(0)\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.filter((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toHaveLength(0);
  });

  it('does NOT flag saturating_add (inline)', () => {
    const src = wrap('  pub fn safe_add(_env: Env, a: u32, b: u32) -> u32 {\n    a.saturating_add(b)\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.filter((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toHaveLength(0);
  });

  it('does NOT flag unchecked arithmetic outside a contractimpl block (inline)', () => {
    const src = `${SOROBAN_HEADER}\nfn helper(a: u32, b: u32) -> u32 { a + b }`;
    const findings = analyzeSorobanSource(src);
    expect(findings.filter((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toHaveLength(0);
  });

  it('flags unchecked arithmetic inside contractimpl (fixture)', () => {
    const src = fixture('overflow_contract.rs');
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toBe(true);
  });

  it('S003 does not fire on lines containing checked_ (fixture)', () => {
    const src = fixture('overflow_contract.rs');
    const findings = analyzeSorobanSource(src);
    const overflowFindings = findings.filter((f) => f.code === CODES.ARITHMETIC_OVERFLOW);
    const lines = src.split('\n');
    for (const f of overflowFindings) {
      const lineText = lines[f.line - 1] ?? '';
      expect(lineText).not.toMatch(/checked_/);
    }
  });

  it('does not flag arithmetic outside contractimpl (fixture)', () => {
    const src = 'fn helper(a: i128, b: i128) -> i128 { a + b }';
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// SEVERITY_ORDER
// ---------------------------------------------------------------------------

describe('SEVERITY_ORDER', () => {
  it('ranks information < warning < error', () => {
    expect(SEVERITY_ORDER.information).toBeLessThan(SEVERITY_ORDER.warning);
    expect(SEVERITY_ORDER.warning).toBeLessThan(SEVERITY_ORDER.error);
  });
});

// ---------------------------------------------------------------------------
// filterBySeverity
// ---------------------------------------------------------------------------

describe('filterBySeverity', () => {
  const mixed: EditorFinding[] = [
    { line: 1, code: 'S002', severity: 'error', message: 'panic' },
    { line: 2, code: 'S001', severity: 'warning', message: 'auth gap' },
    { line: 3, code: 'S003', severity: 'information', message: 'hint' },
  ];

  it('passes all findings when minSeverity is information', () => {
    expect(filterBySeverity(mixed, 'information')).toHaveLength(3);
  });

  it('drops information-level findings when minSeverity is warning', () => {
    const result = filterBySeverity(mixed, 'warning');
    expect(result).toHaveLength(2);
    expect(result.every((f) => f.severity !== 'information')).toBe(true);
  });

  it('keeps only errors when minSeverity is error', () => {
    const result = filterBySeverity(mixed, 'error');
    expect(result).toHaveLength(1);
    expect(result[0].code).toBe('S002');
  });

  it('returns empty array when no findings meet threshold', () => {
    const infoOnly: EditorFinding[] = [{ line: 1, code: 'S003', severity: 'information', message: 'hint' }];
    expect(filterBySeverity(infoOnly, 'error')).toEqual([]);
  });

  it('returns empty array for empty input', () => {
    expect(filterBySeverity([], 'warning')).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Deduplication
// ---------------------------------------------------------------------------

describe('deduplication', () => {
  it('does not emit the same finding twice for the same line + code (inline)', () => {
    const src = wrap('  pub fn double(_env: Env) {\n    panic!("a");\n    panic!("a");\n  }');
    const findings = analyzeSorobanSource(src);
    const panics = findings.filter((f) => f.code === CODES.PANIC_USAGE);
    const lines = new Set(panics.map((f) => f.line));
    expect(panics.length).toBe(lines.size);
  });

  it('does not emit duplicate findings for the same line and code (fixture)', () => {
    const src = [
      'use soroban_sdk::contractimpl;',
      'struct C;',
      '#[contractimpl]',
      'impl C {',
      '  pub fn bad(a: i128, b: i128) -> i128 { a + b }',
      '}',
    ].join('\n');
    const findings = analyzeSorobanSource(src);
    const keys = findings.map((f) => `${f.line}:${f.code}:${f.message.slice(0, 40)}`);
    expect(keys.length).toBe(new Set(keys).size);
  });
});

// ---------------------------------------------------------------------------
// Race condition / stale results handling
// ---------------------------------------------------------------------------

describe('analyzeSorobanSource – race condition resilience', () => {
  /**
   * Tests that analyzeSorobanSource is a pure, synchronous function that doesn't
   * maintain internal state between invocations. This ensures that the extension's
   * debounce + content-cache mechanism (in extension.ts) can safely discard stale
   * in-flight analysis by comparing document text before/after.
   *
   * Regression test for: "results can arrive out of order and show stale diagnostics
   * for an already-changed file" (issue #XXX).
   */
  it('returns consistent results for the same input regardless of call order', () => {
    const versionA = wrap('  pub fn a(_env: Env) { panic!("v1"); }');
    const versionB = wrap('  pub fn b(_env: Env) { panic!("v2"); }');

    // Simulate rapid edits: analyze A, then B, then A again
    const resultA1 = analyzeSorobanSource(versionA);
    const resultB = analyzeSorobanSource(versionB);
    const resultA2 = analyzeSorobanSource(versionA);

    // A's results must be identical regardless of B being analyzed in between
    expect(resultA1).toEqual(resultA2);

    // A and B results must differ (different panic messages/lines)
    expect(resultA1).not.toEqual(resultB);
  });

  it('does not cache findings across invocations (pure function)', () => {
    const src1 = wrap('  pub fn test(_env: Env) { panic!("first"); }');
    const src2 = wrap('  pub fn test(_env: Env, user: Address) {\n    env.storage().persistent().set(&"k", &1);\n  }');

    const findings1 = analyzeSorobanSource(src1);
    const findings2 = analyzeSorobanSource(src2);

    // First call finds panic
    expect(findings1.some((f) => f.code === CODES.PANIC_USAGE)).toBe(true);
    expect(findings1.some((f) => f.code === CODES.AUTH_GAP)).toBe(false);

    // Second call finds auth gap (no panic)
    expect(findings2.some((f) => f.code === CODES.PANIC_USAGE)).toBe(false);
    expect(findings2.some((f) => f.code === CODES.AUTH_GAP)).toBe(true);

    // Results must be independent
    expect(findings1).not.toEqual(findings2);
  });

  it('handles rapid sequential calls with varying input lengths', () => {
    const inputs = [
      wrap('  pub fn a(_env: Env) { panic!("1"); }'),
      wrap('  pub fn b(_env: Env) { panic!("2"); }\n  pub fn c(_env: Env) { panic!("3"); }'),
      wrap('  pub fn d(_env: Env) { panic!("4"); }'),
    ];

    const results = inputs.map(analyzeSorobanSource);

    // Each result should correspond to its input
    expect(results[0].length).toBe(1); // one panic
    expect(results[1].length).toBe(2); // two panics
    expect(results[2].length).toBe(1); // one panic

    // Verify line numbers are correct (no bleed-through from previous analysis)
    expect(results[0][0].line).toBeLessThan(10);
    expect(results[2][0].line).toBeLessThan(10);
  });

  /**
   * Documents the expected behavior when the extension's content cache prevents
   * redundant re-analysis. While analyzeSorobanSource itself doesn't implement
   * caching, the extension.ts layer does (see contentCache Map in extension.ts).
   *
   * This test verifies the function is stable across identical inputs, which is
   * required for the content-based cache to work correctly.
   */
  it('returns identical results for identical inputs (cache-friendliness)', () => {
    const src = wrap('  pub fn test(_env: Env) { panic!("test"); }');

    const result1 = analyzeSorobanSource(src);
    const result2 = analyzeSorobanSource(src);
    const result3 = analyzeSorobanSource(src);

    expect(result1).toEqual(result2);
    expect(result2).toEqual(result3);
  });
});

// ---------------------------------------------------------------------------
// Quick-fix helpers
// ---------------------------------------------------------------------------

describe('buildRequireAuthInsertLine', () => {
  it('returns the line after the opening brace', () => {
    const lines = [
      '  pub fn privileged(env: Env, caller: Address) {',
      '    env.storage().persistent().set(&"key", &42);',
      '  }',
    ];
    expect(buildRequireAuthInsertLine(lines, 0)).toBe(1);
  });

  it('handles multi-line function signatures', () => {
    const lines = [
      '  pub fn privileged(',
      '    env: Env,',
      '    caller: Address',
      '  ) {',
      '    env.storage().persistent().set(&"key", &42);',
      '  }',
    ];
    expect(buildRequireAuthInsertLine(lines, 0)).toBe(4);
  });

  it('returns null when brace not found within window', () => {
    const lines = ['  pub fn incomplete(env: Env'];
    expect(buildRequireAuthInsertLine(lines, 0)).toBeNull();
  });
});

describe('buildUnwrapFix', () => {
  it('replaces .unwrap() with ?', () => {
    const line = '    let val = opt.unwrap();';
    expect(buildUnwrapFix(line)).toBe('    let val = opt?;');
  });

  it('returns null when line has no .unwrap()', () => {
    const line = '    let val = opt?;';
    expect(buildUnwrapFix(line)).toBeNull();
  });

  it('does not replace .unwrap() in comments', () => {
    const line = '    // opt.unwrap() is bad';
    expect(buildUnwrapFix(line)).toBeNull();
  });
});

describe('buildCheckedArithFix', () => {
  it('replaces a + b with checked_add', () => {
    const line = '    let sum = a + b;';
    expect(buildCheckedArithFix(line)).toBe('    let sum = a.checked_add(b).unwrap_or_default();');
  });

  it('replaces a - b with checked_sub', () => {
    const line = '    let diff = a - b;';
    expect(buildCheckedArithFix(line)).toBe('    let diff = a.checked_sub(b).unwrap_or_default();');
  });

  it('returns null when no unchecked arithmetic is found', () => {
    const line = '    let safe = a.checked_add(b).unwrap_or(0);';
    expect(buildCheckedArithFix(line)).toBeNull();
  });

  it('does not replace arithmetic in comments', () => {
    const line = '    // a + b might overflow';
    expect(buildCheckedArithFix(line)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Performance budget
// ---------------------------------------------------------------------------

describe('analyzeSorobanSource – performance budget', () => {
  it('analyzes a 500-line contract in under 100ms', () => {
    const fns = Array.from(
      { length: 90 },
      (_, i) =>
        `  pub fn fn_${i}(env: Env, user: Address, val: i128) -> i128 {\n` +
        `    user.require_auth();\n` +
        `    val.checked_add(1).unwrap_or(0)\n` +
        `  }`,
    ).join('\n');
    const src = `#[contractimpl]\nimpl BigContract {\n${fns}\n}`;
    const start = performance.now();
    analyzeSorobanSource(src);
    const elapsed = performance.now() - start;
    expect(elapsed).toBeLessThan(100);
  });

  it('handles empty input without throwing', () => {
    expect(() => analyzeSorobanSource('')).not.toThrow();
  });

  it('handles very long single line without throwing', () => {
    const src = `fn foo() { ${'x'.repeat(10_000)} }`;
    expect(() => analyzeSorobanSource(src)).not.toThrow();
  });
});

