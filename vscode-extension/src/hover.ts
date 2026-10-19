import * as vscode from 'vscode';
import type { EditorFinding } from './types';

const RULE_DOCS_BASE = 'https://github.com/HyperSafeD/Sanctifier/blob/main/docs/rules';

const CODE_TO_MARKDOWN: Record<string, { title: string; description: string; fix: string; isZk?: boolean }> = {
  // ── S-rules (Soroban / standard) ────────────────────────────────────────────
  S001: {
    title: 'Authentication Gap',
    description: 'A public function performs a privileged operation (storage mutation or cross-contract call) without `require_auth`. Anyone can invoke it.',
    fix: 'caller.require_auth();\n// or\ncaller.require_auth_for_args(().into());',
  },
  S002: {
    title: 'Panic Usage',
    description: '`panic!` aborts the entire contract with no recovery path. Prefer returning a `Result` or using structured errors.',
    fix: '// Instead of panic!("reason")\nreturn Err(Error::MyError);',
  },
  S003: {
    title: 'Arithmetic Overflow',
    description: 'Unchecked `+`, `-`, or `*` between identifiers may overflow or underflow, causing silent incorrect results or panics.',
    fix: '// Use checked/saturating arithmetic\na.checked_add(b).ok_or(Error::Overflow)?;\n// or\na.saturating_add(b)',
  },
  S006: {
    title: 'Unsafe Pattern',
    description: '`.unwrap()` or `.expect()` can abort the contract if the precondition fails. Use explicit error handling instead.',
    fix: '// Instead of val.unwrap()\nmatch val {\n    Some(v) => v,\n    None => return Err(Error::NotFound),\n}',
  },
  // ── Z-rules (ZK circuit) ────────────────────────────────────────────────────
  Z001: {
    isZk: true,
    title: 'Under-constrained Signal',
    description: 'A circuit signal is used in an output or assertion but lacks sufficient constraints to be uniquely determined. An under-constrained signal allows a malicious prover to choose arbitrary values, defeating the soundness guarantee of the proof.',
    fix: '// Add explicit range or equality constraints for each signal:\n// component eq = IsEqual();\n// eq.in[0] <== signal;\n// eq.in[1] <== expected;',
  },
  Z002: {
    isZk: true,
    title: 'Missing Nullifier Check',
    description: 'A spend or claim path does not verify a nullifier, allowing the same witness to produce valid proofs multiple times (double-spend). Every private input that represents a unique spend must commit to an on-chain nullifier.',
    fix: '// Compute nullifier commitment and verify uniqueness:\n// signal nullifier <== Poseidon(2)([secret, nonce]);\n// nullifiers[nullifier] === 0; // check not spent\n// nullifiers[nullifier] <== 1; // mark spent',
  },
  Z003: {
    isZk: true,
    title: 'Unconstrained Public Input',
    description: 'A public input to the circuit is not tied to any constraint, so any value passes verification. This breaks the binding property: the verifier cannot trust what the prover claims the public input represents.',
    fix: '// Bind public inputs to internal signals via constraints:\n// signal input pub_value;\n// internal_signal === pub_value; // enforce binding',
  },
  Z007: {
    isZk: true,
    title: 'Missing Range Check',
    description: 'A numeric signal is not bounded to an expected range of values. Without a range check, a prover may supply out-of-range values (e.g., negative balances encoded as large field elements) that pass all other constraints.',
    fix: '// Use a range check component:\n// component range = Num2Bits(64);\n// range.in <== value;\n// // This constrains value to [0, 2^64)',
  },
  Z008: {
    isZk: true,
    title: 'Arithmetic Soundness (ZK)',
    description: 'An arithmetic operation inside the circuit is not sound: either it can overflow the field prime, produce an ambiguous result, or relies on native integer arithmetic assumptions that do not hold inside a finite field.',
    fix: '// Perform arithmetic in the field explicitly:\n// // Avoid: a * b where result may exceed field prime\n// // Prefer: split into constrained sub-operations\n// component mul = Multiplier();\n// mul.a <== a;\n// mul.b <== b;',
  },
  Z010: {
    isZk: true,
    title: 'Missing Auth Constraint (ZK)',
    description: 'A circuit that gates access to a privileged action does not verify the authority signal (e.g., a hash of a secret key). Anyone who can construct a valid witness can bypass the intended authorization.',
    fix: '// Verify authority in the circuit:\n// signal input secret;\n// signal pubKeyHash <== Poseidon(1)([secret]);\n// pubKeyHash === knownPubKeyHash; // constrain auth',
  },
};

/** Returns true when the rule code belongs to the ZK family (Z-rules, issue #1239). */
export function isZkRule(code: string): boolean {
  return /^Z\d+$/.test(code);
}

function getMarkdownContent(finding: EditorFinding): vscode.MarkdownString {
  const meta = CODE_TO_MARKDOWN[finding.code] ?? {
    title: finding.code,
    description: finding.message,
    fix: 'See rule documentation for suggested fix.',
    isZk: isZkRule(finding.code),
  };

  const severityIcon = finding.severity === 'error' ? '$(error)' : finding.severity === 'warning' ? '$(warning)' : '$(info)';
  const severityLabel = finding.severity.charAt(0).toUpperCase() + finding.severity.slice(1);
  const zkBadge = meta.isZk ? ' 🔒 **ZK**' : '';

  const markdown = new vscode.MarkdownString();
  markdown.isTrusted = true;
  markdown.supportHtml = true;

  markdown.appendMarkdown(`### ${severityIcon} ${meta.title} (\`${finding.code}\`)${zkBadge}\n\n`);
  if (meta.isZk) {
    markdown.appendMarkdown(`> *ZK circuit finding — this rule applies to zero-knowledge constraint systems, not Soroban contract code.*\n\n`);
  }
  markdown.appendMarkdown(`**Severity:** ${severityLabel}\n\n`);
  markdown.appendMarkdown(`${meta.description}\n\n`);
  markdown.appendMarkdown(`---\n\n`);
  markdown.appendMarkdown(`**Suggested fix:**\n\n`);
  markdown.appendCodeblock(meta.fix, meta.isZk ? 'circom' : 'rust');
  markdown.appendMarkdown(`\n\n`);
  markdown.appendMarkdown(
    `[Learn more](${RULE_DOCS_BASE}/${finding.code.toLowerCase()}.md) &nbsp;|&nbsp; ` +
    `[Suppress](command:sanctifier.suppressFinding?${encodeURIComponent(JSON.stringify({ code: finding.code, line: finding.line }))})`,
  );

  return markdown;
}

export class SanctifierHoverProvider implements vscode.HoverProvider {
  private findingsMap: Map<string, EditorFinding[]>;

  constructor(findingsMap: Map<string, EditorFinding[]>) {
    this.findingsMap = findingsMap;
  }

  provideHover(
    document: vscode.TextDocument,
    position: vscode.Position,
    _token: vscode.CancellationToken,
  ): vscode.ProviderResult<vscode.Hover> {
    const key = document.uri.toString();
    const findings = this.findingsMap.get(key);
    if (!findings || findings.length === 0) return null;

    const line = position.line + 1;
    const match = findings.find((f) => f.line === line);
    if (!match) return null;

    const range = new vscode.Range(
      new vscode.Position(Math.max(0, match.line - 1), 0),
      new vscode.Position(
        match.endLine ? Math.max(match.line - 1, match.endLine - 1) : Math.max(0, match.line - 1),
        match.endCharacter ?? Number.MAX_SAFE_INTEGER,
      ),
    );

    return {
      contents: [getMarkdownContent(match)],
      range,
    };
  }
}
