import {
  buildRequireAuthInsertLine,
  buildUnwrapFix,
  buildCheckedArithFix,
  CODES,
} from './analyzer';
import { SanctifierCodeActionProvider } from './code-actions';
import {
  Range,
  Diagnostic,
  DiagnosticSeverity,
  Uri,
  Position,
  WorkspaceEdit,
} from './__mocks__/vscode';
import type * as vscode from 'vscode';

// ---------------------------------------------------------------------------
// buildRequireAuthInsertLine
// ---------------------------------------------------------------------------

describe('buildRequireAuthInsertLine', () => {
  it('returns the line index after the opening brace', () => {
    const lines = [
      '  pub fn increment(env: Env, user: Address) {',
      '    let x = 1;',
      '  }',
    ];
    expect(buildRequireAuthInsertLine(lines, 0)).toBe(1);
  });

  it('handles brace on a separate line', () => {
    const lines = [
      '  pub fn increment(env: Env, user: Address)',
      '  {',
      '    let x = 1;',
      '  }',
    ];
    expect(buildRequireAuthInsertLine(lines, 0)).toBe(2);
  });

  it('returns null when no opening brace found within window', () => {
    const lines = Array.from({ length: 20 }, (_, i) => `  // line ${i}`);
    expect(buildRequireAuthInsertLine(lines, 0)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// buildUnwrapFix
// ---------------------------------------------------------------------------

describe('buildUnwrapFix', () => {
  it('replaces .unwrap() with ?', () => {
    expect(buildUnwrapFix('    let x = foo.unwrap();')).toBe('    let x = foo?;');
  });

  it('handles whitespace inside unwrap parens', () => {
    expect(buildUnwrapFix('    let x = foo.unwrap( );')).toBe('    let x = foo?;');
  });

  it('returns null when no .unwrap() present', () => {
    expect(buildUnwrapFix('    let x = foo.unwrap_or(0);')).toBeNull();
  });

  it('returns null for a line comment containing unwrap', () => {
    expect(buildUnwrapFix('    // foo.unwrap() bad')).toBeNull();
  });

  it('replaces only the first .unwrap() when multiple appear', () => {
    const result = buildUnwrapFix('    let x = a.unwrap(); let y = b.unwrap();');
    expect(result).toBe('    let x = a?; let y = b.unwrap();');
  });
});

// ---------------------------------------------------------------------------
// buildCheckedArithFix
// ---------------------------------------------------------------------------

describe('buildCheckedArithFix', () => {
  it('replaces identifier + identifier with checked_add', () => {
    const result = buildCheckedArithFix('    let next = current + by;');
    expect(result).toContain('checked_add');
    expect(result).toContain('current');
    expect(result).toContain('by');
  });

  it('replaces identifier - identifier with checked_sub', () => {
    const result = buildCheckedArithFix('    let diff = total - fee;');
    expect(result).toContain('checked_sub');
    expect(result).toContain('total');
    expect(result).toContain('fee');
  });

  it('returns null when arithmetic already uses checked_add', () => {
    expect(buildCheckedArithFix('    let x = a.checked_add(b).unwrap_or(0);')).toBeNull();
  });

  it('returns null when no arithmetic operator is present', () => {
    expect(buildCheckedArithFix('    let x = foo.bar();')).toBeNull();
  });

  it('returns null when the + is in a comment', () => {
    expect(buildCheckedArithFix('    // a + b')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// SanctifierCodeActionProvider — unit tests with mocked document
// ---------------------------------------------------------------------------

function makeDocument(lines: string[]): import('vscode').TextDocument {
  const text = lines.join('\n');
  return {
    getText: () => text,
    lineAt: (idx: number) => ({
      text: lines[idx] ?? '',
      range: new Range(idx, 0, idx, (lines[idx] ?? '').length),
    }),
    uri: Uri.file('/fake/contract.rs'),
  } as unknown as import('vscode').TextDocument;
}

function makeDiag(code: string, line: number, source = 'sanctifier'): Diagnostic {
  const diag = new Diagnostic(new Range(line, 0, line, 0), 'test finding', DiagnosticSeverity.Warning);
  diag.code = code;
  diag.source = source;
  return diag;
}

describe('SanctifierCodeActionProvider', () => {
  const provider = new SanctifierCodeActionProvider();
  const emptyRange = new Range(0, 0, 0, 0) as unknown as vscode.Range;
  const emptyContext = (diags: Diagnostic[]) =>
    ({ diagnostics: diags } as unknown as vscode.CodeActionContext);

  it('returns no actions for non-sanctifier diagnostics', () => {
    const doc = makeDocument(['let x = 1;']);
    const diag = makeDiag('S001', 0, 'eslint');
    const actions = provider.provideCodeActions(doc, emptyRange, emptyContext([diag]));
    expect(actions).toHaveLength(0);
  });

  it('returns require_auth action for S001 when function has an opening brace', () => {
    const doc = makeDocument([
      '  pub fn transfer(env: Env, user: Address) {',
      '    env.storage().persistent().set(&"k", &1u32);',
      '  }',
    ]);
    const diag = makeDiag(CODES.AUTH_GAP, 0);
    const actions = provider.provideCodeActions(doc, emptyRange, emptyContext([diag]));
    expect(actions).toHaveLength(1);
    expect(actions[0].title).toContain('require_auth');
    const edit = actions[0].edit as unknown as WorkspaceEdit;
    expect(edit._inserts).toHaveLength(1);
    expect(edit._inserts[0].position).toEqual(new Position(1, 0));
    expect(edit._inserts[0].text).toContain('require_auth');
  });

  it('returns no action for S001 when brace cannot be found', () => {
    const lines = ['  pub fn no_brace(env: Env)', ...Array(20).fill('  // comment')];
    const doc = makeDocument(lines);
    const diag = makeDiag(CODES.AUTH_GAP, 0);
    const actions = provider.provideCodeActions(doc, emptyRange, emptyContext([diag]));
    expect(actions).toHaveLength(0);
  });

  it('returns unwrap->? action for S006', () => {
    const doc = makeDocument([
      '    let val = storage.get(&key).unwrap();',
    ]);
    const diag = makeDiag(CODES.UNSAFE_PATTERN, 0);
    const actions = provider.provideCodeActions(doc, emptyRange, emptyContext([diag]));
    expect(actions).toHaveLength(1);
    expect(actions[0].title).toContain('?');
    const edit = actions[0].edit as unknown as WorkspaceEdit;
    expect(edit._replaces[0].text).not.toContain('.unwrap()');
    expect(edit._replaces[0].text).toContain('?');
  });

  it('returns no action for S002 (panic!) when no unwrap on line', () => {
    const doc = makeDocument(['    panic!("boom");']);
    const diag = makeDiag(CODES.PANIC_USAGE, 0);
    const actions = provider.provideCodeActions(doc, emptyRange, emptyContext([diag]));
    // panic! has no unwrap to replace — provider returns no action
    expect(actions).toHaveLength(0);
  });

  it('returns checked_add action for S003 with + operator', () => {
    const doc = makeDocument(['    let next = current + by;']);
    const diag = makeDiag(CODES.ARITHMETIC_OVERFLOW, 0);
    const actions = provider.provideCodeActions(doc, emptyRange, emptyContext([diag]));
    expect(actions).toHaveLength(1);
    expect(actions[0].title).toContain('checked_add');
    const edit = actions[0].edit as unknown as WorkspaceEdit;
    expect(edit._replaces[0].text).toContain('checked_add');
  });

  it('returns checked_sub action for S003 with - operator', () => {
    const doc = makeDocument(['    let diff = total - fee;']);
    const diag = makeDiag(CODES.ARITHMETIC_OVERFLOW, 0);
    const actions = provider.provideCodeActions(doc, emptyRange, emptyContext([diag]));
    expect(actions).toHaveLength(1);
    expect(actions[0].title).toContain('checked_sub');
  });

  it('handles multiple diagnostics in one pass', () => {
    const doc = makeDocument([
      '  pub fn bad(env: Env, user: Address) {',
      '    let next = current + by;',
      '    let val = foo.unwrap();',
      '  }',
    ]);
    const diags = [
      makeDiag(CODES.AUTH_GAP, 0),
      makeDiag(CODES.ARITHMETIC_OVERFLOW, 1),
      makeDiag(CODES.UNSAFE_PATTERN, 2),
    ];
    const actions = provider.provideCodeActions(doc, emptyRange, emptyContext(diags));
    expect(actions).toHaveLength(3);
  });
});
