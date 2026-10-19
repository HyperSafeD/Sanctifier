import * as fs from 'fs';
import * as path from 'path';
import {
  analyzeSorobanSource,
  looksLikeSorobanSource,
  filterBySeverity,
  CODES,
  SEVERITY_ORDER,
} from '../analyzer';

const fixturesDir = path.join(__dirname, '..', '..', 'src', 'test', 'fixtures');

function fixture(name: string): string {
  return fs.readFileSync(path.join(fixturesDir, name), 'utf8');
}

describe('looksLikeSorobanSource (fixture-based)', () => {
  it('detects #[contractimpl] attribute', () => {
    expect(looksLikeSorobanSource('#[contractimpl]\nimpl Foo {}')).toBe(true);
  });

  it('detects soroban_sdk crate usage', () => {
    expect(looksLikeSorobanSource('use soroban_sdk::Env;')).toBe(true);
  });

  it('detects #[contract] attribute', () => {
    expect(looksLikeSorobanSource('#[contract]\nstruct Foo;')).toBe(true);
  });

  it('returns false for plain Rust with no Soroban markers', () => {
    expect(looksLikeSorobanSource('fn main() {\n  println!("hello");\n}')).toBe(false);
  });
});

describe('analyzeSorobanSource — auth gap (S001) via fixture', () => {
  it('flags a privileged fn missing require_auth', () => {
    const src = fixture('auth_gap.rs');
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.AUTH_GAP)).toBe(true);
  });

  it('does not flag when require_auth is present', () => {
    const src = fixture('safe_contract.rs');
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.AUTH_GAP)).toBe(false);
  });
});

describe('analyzeSorobanSource — unsafe patterns (S002 / S006) via fixture', () => {
  it('flags panic! with S002', () => {
    const src = fixture('panic_contract.rs');
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.PANIC_USAGE)).toBe(true);
  });

  it('flags .unwrap() with S006', () => {
    const src = fixture('panic_contract.rs');
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.UNSAFE_PATTERN)).toBe(true);
  });

  it('does not flag commented-out unwrap', () => {
    const src = '// let x = val.unwrap();\nfn safe() {}';
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.UNSAFE_PATTERN)).toBe(false);
  });
});

describe('analyzeSorobanSource — arithmetic overflow (S003) via fixture', () => {
  it('flags unchecked arithmetic inside contractimpl', () => {
    const src = fixture('overflow_contract.rs');
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toBe(true);
  });

  it('S003 does not fire on lines containing checked_', () => {
    const src = fixture('overflow_contract.rs');
    const findings = analyzeSorobanSource(src);
    const overflowFindings = findings.filter((f) => f.code === CODES.ARITHMETIC_OVERFLOW);
    const lines = src.split('\n');
    for (const f of overflowFindings) {
      const lineText = lines[f.line - 1] ?? '';
      expect(lineText).not.toMatch(/checked_/);
    }
  });

  it('does not flag arithmetic outside contractimpl', () => {
    const src = 'fn helper(a: i128, b: i128) -> i128 { a + b }';
    const findings = analyzeSorobanSource(src);
    expect(findings.some((f) => f.code === CODES.ARITHMETIC_OVERFLOW)).toBe(false);
  });
});

describe('analyzeSorobanSource — deduplication via fixture', () => {
  it('does not emit duplicate findings for the same line and code', () => {
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

describe('SEVERITY_ORDER (integration)', () => {
  it('ranks information < warning < error', () => {
    expect(SEVERITY_ORDER.information).toBeLessThan(SEVERITY_ORDER.warning);
    expect(SEVERITY_ORDER.warning).toBeLessThan(SEVERITY_ORDER.error);
  });
});

describe('filterBySeverity (integration)', () => {
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
