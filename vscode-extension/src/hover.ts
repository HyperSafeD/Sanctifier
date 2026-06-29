import * as vscode from 'vscode';
import type { EditorFinding } from './types';

const RULE_DOCS_BASE = 'https://github.com/HyperSafeD/Sanctifier/blob/main/docs/rules';

const CODE_TO_MARKDOWN: Record<string, { title: string; description: string; fix: string }> = {
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
};

function getMarkdownContent(finding: EditorFinding): vscode.MarkdownString {
  const meta = CODE_TO_MARKDOWN[finding.code] ?? {
    title: finding.code,
    description: finding.message,
    fix: 'See rule documentation for suggested fix.',
  };

  const severityIcon = finding.severity === 'error' ? '$(error)' : finding.severity === 'warning' ? '$(warning)' : '$(info)';
  const severityLabel = finding.severity.charAt(0).toUpperCase() + finding.severity.slice(1);

  const markdown = new vscode.MarkdownString();
  markdown.isTrusted = true;
  markdown.supportHtml = true;

  markdown.appendMarkdown(`### ${severityIcon} ${meta.title} (\`${finding.code}\`)\n\n`);
  markdown.appendMarkdown(`**Severity:** ${severityLabel}\n\n`);
  markdown.appendMarkdown(`${meta.description}\n\n`);
  markdown.appendMarkdown(`---\n\n`);
  markdown.appendMarkdown(`**Suggested fix:**\n\n`);
  markdown.appendCodeblock(meta.fix, 'rust');
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
