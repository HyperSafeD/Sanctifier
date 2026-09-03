import { analyzeSorobanSource, looksLikeSorobanSource, filterBySeverity, CODES, SEVERITY_ORDER } from './analyzer';

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
// S002 – PANIC_USAGE
// ---------------------------------------------------------------------------

describe('S002 PANIC_USAGE', () => {
  it('flags panic! inside a contract', () => {
    const src = wrap('  pub fn blow_up(env: Env) {\n    panic!("never");\n  }');
    const findings = analyzeSorobanSource(src);
    const hit = findings.find((f) => f.code === CODES.PANIC_USAGE);
    expect(hit).toBeDefined();
    expect(hit?.severity).toBe('error');
  });

  it('does NOT flag panic! in a line comment', () => {
    const src = wrap('  pub fn ok(_env: Env) -> u32 {\n    // panic! is bad\n    42\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.filter((f) => f.code === CODES.PANIC_USAGE)).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// S006 – UNSAFE_PATTERN (.unwrap / .expect)
// ---------------------------------------------------------------------------

describe('S006 UNSAFE_PATTERN', () => {
  it('flags .unwrap() inside a contract function', () => {
    const src = wrap('  pub fn risky(_env: Env) -> u32 {\n    let val: Option<u32> = Some(1);\n    val.unwrap()\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.find((f) => f.code === CODES.UNSAFE_PATTERN)).toBeDefined();
  });

  it('flags .expect("…") inside a contract function', () => {
    const src = wrap('  pub fn risky(_env: Env) -> u32 {\n    let val: Option<u32> = None;\n    val.expect("must exist")\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.find((f) => f.code === CODES.UNSAFE_PATTERN)).toBeDefined();
  });

  it('does NOT flag .unwrap() inside a line comment', () => {
    const src = wrap('  pub fn safe(_env: Env) -> u32 {\n    // val.unwrap() is bad\n    42\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.filter((f) => f.code === CODES.UNSAFE_PATTERN)).toHaveLength(0);
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
  it('flags a pub fn that mutates storage without require_auth', () => {
    const findings = analyzeSorobanSource(AUTH_GAP_SRC);
    expect(findings.find((f) => f.code === CODES.AUTH_GAP)).toBeDefined();
  });

  it('does NOT flag a pub fn that calls require_auth before mutating storage', () => {
    const findings = analyzeSorobanSource(AUTH_OK_SRC);
    expect(findings.filter((f) => f.code === CODES.AUTH_GAP)).toHaveLength(0);
  });

  it('does not flag when require_auth_for_args is present', () => {
    const gaps = analyzeSorobanSource(AUTH_FOR_ARGS_SRC).filter((f) => f.code === CODES.AUTH_GAP);
    expect(gaps).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// S003 – ARITHMETIC_OVERFLOW
// ---------------------------------------------------------------------------

describe('S003 ARITHMETIC_OVERFLOW', () => {
  it('flags unchecked + between identifiers inside contractimpl', () => {
    const src = wrap('  pub fn add(_env: Env, a: u32, b: u32) -> u32 {\n    a + b\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.find((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toBeDefined();
  });

  it('does NOT flag checked_add', () => {
    const src = wrap('  pub fn safe_add(_env: Env, a: u32, b: u32) -> u32 {\n    a.checked_add(b).unwrap_or(0)\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.filter((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toHaveLength(0);
  });

  it('does NOT flag saturating_add', () => {
    const src = wrap('  pub fn safe_add(_env: Env, a: u32, b: u32) -> u32 {\n    a.saturating_add(b)\n  }');
    const findings = analyzeSorobanSource(src);
    expect(findings.filter((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toHaveLength(0);
  });

  it('does NOT flag unchecked arithmetic outside a contractimpl block', () => {
    const src = `${SOROBAN_HEADER}\nfn helper(a: u32, b: u32) -> u32 { a + b }`;
    const findings = analyzeSorobanSource(src);
    expect(findings.filter((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toHaveLength(0);
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
  const mixed = [
    { line: 1, code: 'S002', severity: 'error' as const, message: 'panic' },
    { line: 2, code: 'S001', severity: 'warning' as const, message: 'auth gap' },
    { line: 3, code: 'S003', severity: 'information' as const, message: 'hint' },
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
    const infoOnly = [{ line: 1, code: 'S003', severity: 'information' as const, message: 'hint' }];
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
  it('does not emit the same finding twice for the same line + code', () => {
    const src = wrap('  pub fn double(_env: Env) {\n    panic!("a");\n    panic!("a");\n  }');
    const findings = analyzeSorobanSource(src);
    const panics = findings.filter((f) => f.code === CODES.PANIC_USAGE);
    const lines = new Set(panics.map((f) => f.line));
    expect(panics.length).toBe(lines.size);
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
