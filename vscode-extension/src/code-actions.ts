import * as vscode from 'vscode';
import {
  CODES,
  buildRequireAuthInsertLine,
  buildUnwrapFix,
  buildCheckedArithFix,
} from './analyzer';

const SOURCE = 'sanctifier';

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
}
