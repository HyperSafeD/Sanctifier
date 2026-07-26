import * as vscode from 'vscode';
import {
  CODES,
  buildRequireAuthInsertLine,
  buildUnwrapFix,
  buildCheckedArithFix,
} from './analyzer';
import { isZkRule } from './hover';

const SOURCE = 'sanctifier';

// Z-rule codes that have well-defined mechanical quick-fixes (issue #1239).
// Rules requiring deep contextual judgment (Z007 range-check design, etc.)
// provide hover-only explanations; they are not listed here.
const ZK_QUICK_FIX_CODES = new Set(['Z010', 'Z002']);

export class SanctifierCodeActionProvider implements vscode.CodeActionProvider {
  static readonly providedCodeActionKinds = [vscode.CodeActionKind.QuickFix];

  provideCodeActions(
    document: vscode.TextDocument,
    _range: vscode.Range | vscode.Selection,
    context: vscode.CodeActionContext,
  ): vscode.CodeAction[] {
    const actions: vscode.CodeAction[] = [];

    for (const diag of context.diagnostics) {
      if (diag.source !== SOURCE) {
        continue;
      }
      const code = String(
        typeof diag.code === 'object' && diag.code !== null
          ? (diag.code as { value: string | number }).value
          : (diag.code ?? ''),
      );

      if (code === CODES.AUTH_GAP) {
        const action = this.buildRequireAuthAction(document, diag);
        if (action) {
          actions.push(action);
        }
      } else if (code === CODES.PANIC_USAGE || code === CODES.UNSAFE_PATTERN) {
        const action = this.buildUnwrapAction(document, diag);
        if (action) {
          actions.push(action);
        }
      } else if (code === CODES.ARITHMETIC_OVERFLOW) {
        const action = this.buildArithAction(document, diag);
        if (action) {
          actions.push(action);
        }
      } else if (isZkRule(code) && ZK_QUICK_FIX_CODES.has(code)) {
        const action = this.buildZkQuickFixAction(document, diag, code);
        if (action) {
          actions.push(action);
        }
      }
    }

    return actions;
  }

  private buildRequireAuthAction(
    document: vscode.TextDocument,
    diag: vscode.Diagnostic,
  ): vscode.CodeAction | null {
    const lines = document.getText().split(/\r?\n/);
    const fnLineIdx = diag.range.start.line;
    const insertLineIdx = buildRequireAuthInsertLine(lines, fnLineIdx);
    if (insertLineIdx === null) {
      return null;
    }

    const indent = (lines[fnLineIdx].match(/^(\s*)/) ?? ['', ''])[1];
    const action = new vscode.CodeAction(
      'Insert address.require_auth();',
      vscode.CodeActionKind.QuickFix,
    );
    action.diagnostics = [diag];
    action.isPreferred = true;
    action.edit = new vscode.WorkspaceEdit();
    action.edit.insert(
      document.uri,
      new vscode.Position(insertLineIdx, 0),
      `${indent}    address.require_auth();\n`,
    );
    return action;
  }

  private buildUnwrapAction(
    document: vscode.TextDocument,
    diag: vscode.Diagnostic,
  ): vscode.CodeAction | null {
    const lineIdx = diag.range.start.line;
    const lineText = document.lineAt(lineIdx).text;
    const fixed = buildUnwrapFix(lineText);
    if (fixed === null) {
      return null;
    }

    const action = new vscode.CodeAction(
      'Replace .unwrap() with ?',
      vscode.CodeActionKind.QuickFix,
    );
    action.diagnostics = [diag];
    action.isPreferred = true;
    action.edit = new vscode.WorkspaceEdit();
    action.edit.replace(document.uri, document.lineAt(lineIdx).range, fixed);
    return action;
  }

  private buildArithAction(
    document: vscode.TextDocument,
    diag: vscode.Diagnostic,
  ): vscode.CodeAction | null {
    const lineIdx = diag.range.start.line;
    const lineText = document.lineAt(lineIdx).text;
    const fixed = buildCheckedArithFix(lineText);
    if (fixed === null) {
      return null;
    }

    const isSubtraction = /\b[a-zA-Z_]\w*\s*-\s*[a-zA-Z_]\w*\b/.test(lineText.split('//')[0]);
    const action = new vscode.CodeAction(
      isSubtraction ? 'Use checked_sub' : 'Use checked_add',
      vscode.CodeActionKind.QuickFix,
    );
    action.diagnostics = [diag];
    action.isPreferred = true;
    action.edit = new vscode.WorkspaceEdit();
    action.edit.replace(document.uri, document.lineAt(lineIdx).range, fixed);
    return action;
  }

  /**
   * Quick-fix scaffolds for Z-rules with well-defined mechanical remediation
   * (issue #1239). Inserts a comment block with the required constraint pattern
   * directly after the flagged line.
   */
  private buildZkQuickFixAction(
    document: vscode.TextDocument,
    diag: vscode.Diagnostic,
    code: string,
  ): vscode.CodeAction | null {
    const lineIdx = diag.range.start.line;
    const lineText = document.lineAt(lineIdx).text;
    const indent = (lineText.match(/^(\s*)/) ?? ['', ''])[1];

    let snippet: string;
    let label: string;

    if (code === 'Z010') {
      label = 'Add ZK auth constraint (Z010)';
      snippet =
        `${indent}// Z010 fix: constrain the authority signal\n` +
        `${indent}// signal pubKeyHash <== Poseidon(1)([secret]);\n` +
        `${indent}// pubKeyHash === knownPubKeyHash;\n`;
    } else if (code === 'Z002') {
      label = 'Add nullifier check scaffold (Z002)';
      snippet =
        `${indent}// Z002 fix: verify nullifier uniqueness before spend\n` +
        `${indent}// signal nullifier <== Poseidon(2)([secret, nonce]);\n` +
        `${indent}// nullifiers[nullifier] === 0;\n` +
        `${indent}// nullifiers[nullifier] <== 1;\n`;
    } else {
      return null;
    }

    const action = new vscode.CodeAction(label, vscode.CodeActionKind.QuickFix);
    action.diagnostics = [diag];
    action.isPreferred = true;
    action.edit = new vscode.WorkspaceEdit();
    action.edit.insert(document.uri, new vscode.Position(lineIdx + 1, 0), snippet);
    return action;
  }
}
