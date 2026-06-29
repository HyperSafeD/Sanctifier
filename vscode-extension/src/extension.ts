import * as vscode from 'vscode';
import { analyzeSorobanSource, looksLikeSorobanSource, filterBySeverity, type Severity } from './analyzer';
import { type EditorFinding, type SanctifierExtensionApi } from './types';
import { folderLooksLikeSorobanProject, invalidateWorkspaceCache } from './workspace';
import { spawn } from 'child_process';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { findingsToSarif, parseSarif, serialiseSarif, validateSarifShape, sarifToFindings } from './sarif';
import { validateSarifContent, validateSarifResultCount, isPathWithinWorkspace, MAX_SARIF_BYTES } from './security';
import { checkAndSuggestDevcontainer } from './devcontainer';
import { SanctifierHoverProvider } from './hover';
import { recordScan, sendTelemetry, promptTelemetryOptIn } from './telemetry';

const SOURCE = 'sanctifier';
const EXTENSION_VERSION: string = (() => {
  try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    return (require('../package.json') as { version: string }).version;
  } catch {
    return '0.0.0';
  }
})();

function getConfig() {
  return vscode.workspace.getConfiguration('sanctifier');
}

function findingToDiagnostic(doc: vscode.TextDocument, f: EditorFinding): vscode.Diagnostic {
  const lineIdx = Math.max(0, Math.min(doc.lineCount - 1, f.line - 1));
  const line = doc.lineAt(lineIdx);
  const range =
    f.endLine !== undefined
      ? new vscode.Range(
          lineIdx,
          0,
          Math.max(lineIdx, f.endLine - 1),
          f.endCharacter ?? Number.MAX_SAFE_INTEGER
        )
      : new vscode.Range(lineIdx, 0, lineIdx, line.range.end.character || line.text.length);

  const sev =
    f.severity === 'error'
      ? vscode.DiagnosticSeverity.Error
      : f.severity === 'information'
        ? vscode.DiagnosticSeverity.Information
        : vscode.DiagnosticSeverity.Warning;

  const d = new vscode.Diagnostic(range, f.message, sev);
  d.code = f.code;
  d.source = SOURCE;
  return d;
}

function validateSanctifierPath(exePath: string): void {
  const trimmed = exePath.trim();
  if (!trimmed) {
    return;
  }
  if (!fs.existsSync(trimmed)) {
    vscode.window.showWarningMessage(
      `Sanctifier: sanctifierPath "${trimmed}" was not found on disk. ` +
        'Update sanctifier.sanctifierPath to a valid CLI binary path.',
    );
  }
}

export async function activate(context: vscode.ExtensionContext): Promise<SanctifierExtensionApi> {
  const collection = vscode.languages.createDiagnosticCollection(SOURCE);
  const outputChannel = vscode.window.createOutputChannel('Sanctifier');

  /** Live snapshot of findings keyed by document URI string. */
  const findingsCache = new Map<string, EditorFinding[]>();
  /** Last-seen text per document — skip re-analysis when content is unchanged. */
  const contentCache = new Map<string, string>();

  const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 10);
  statusBar.command = 'workbench.actions.view.problems';
  statusBar.tooltip = 'Sanctifier — click to open Problems panel';
  context.subscriptions.push(statusBar, collection, outputChannel);

  let scanning = false;

  context.subscriptions.push(
    vscode.languages.registerHoverProvider(
      { language: 'rust' },
      new SanctifierHoverProvider(findingsCache),
    ),
  );

  function updateStatusBar(): void {
    const enabled = getConfig().get<boolean>('enable') ?? true;
    if (!enabled) {
      statusBar.text = '$(shield) Sanctifier: off';
      statusBar.tooltip = 'Sanctifier is disabled — click to open Problems panel';
      statusBar.show();
      return;
    }
    if (scanning) {
      statusBar.text = '$(sync~spin) Sanctifier: scanning…';
      statusBar.tooltip = 'Sanctifier is scanning…';
      statusBar.show();
      return;
    }
    const editor = vscode.window.activeTextEditor;
    if (editor && editor.document.languageId === 'rust') {
      const current = findingsCache.get(editor.document.uri.toString()) ?? [];
      const count = current.length;
      statusBar.text = count > 0
        ? `$(warning) Sanctifier: ${count} finding${count !== 1 ? 's' : ''}`
        : '$(shield) Sanctifier';
      statusBar.tooltip = count > 0
        ? `${count} finding${count !== 1 ? 's' : ''} — click to open Problems panel`
        : 'Sanctifier — click to open Problems panel';
    } else {
      statusBar.text = '$(shield) Sanctifier';
      statusBar.tooltip = 'Sanctifier — click to open Problems panel';
    }
    statusBar.show();
  }

  const debouncers = new Map<string, ReturnType<typeof setTimeout>>();

  const runAnalysis = (doc: vscode.TextDocument) => {
    if (doc.languageId !== 'rust') {
      return;
    }
    const cfg = getConfig();
    if (!cfg.get<boolean>('enable')) {
      collection.delete(doc.uri);
      findingsCache.delete(doc.uri.toString());
      updateStatusBar();
      return;
    }
    const text = doc.getText();
    if (!looksLikeSorobanSource(text)) {
      collection.delete(doc.uri);
      findingsCache.delete(doc.uri.toString());
      updateStatusBar();
      return;
    }

    const key = doc.uri.toString();
    if (contentCache.get(key) === text) {
      return;
    }
    contentCache.set(key, text);

    scanning = true;
    updateStatusBar();

    const minSeverity = (cfg.get<string>('minSeverity') ?? 'warning') as Severity;
    const allFindings = analyzeSorobanSource(text);
    const findings = filterBySeverity(allFindings, minSeverity);

    const byRule: Record<string, number> = {};
    for (const f of findings) {
      byRule[f.code] = (byRule[f.code] ?? 0) + 1;
    }
    recordScan(byRule);

    findingsCache.set(doc.uri.toString(), findings);
    const diags = findings.map((f) => findingToDiagnostic(doc, f));
    collection.set(doc.uri, diags);

    scanning = false;
    updateStatusBar();
  };

  const schedule = (doc: vscode.TextDocument) => {
    const onlySorobanWs = getConfig().get<boolean>('onlyInSorobanWorkspace');
    if (onlySorobanWs) {
      const folder = vscode.workspace.getWorkspaceFolder(doc.uri);
      if (!folder) {
        collection.delete(doc.uri);
        findingsCache.delete(doc.uri.toString());
        updateStatusBar();
        return;
      }
    }
    const ms = Math.min(5000, Math.max(100, getConfig().get<number>('debounceMs') ?? 400));
    const key = doc.uri.toString();
    const prev = debouncers.get(key);
    if (prev) {
      clearTimeout(prev);
    }
    debouncers.set(
      key,
      setTimeout(async () => {
        debouncers.delete(key);
        const requireSoroban = getConfig().get<boolean>('onlyInSorobanWorkspace');
        if (requireSoroban) {
          const folder = vscode.workspace.getWorkspaceFolder(doc.uri);
          if (!folder) {
            collection.delete(doc.uri);
            findingsCache.delete(doc.uri.toString());
            updateStatusBar();
            return;
          }
          const ok = await folderLooksLikeSorobanProject(folder);
          if (!ok) {
            collection.delete(doc.uri);
            findingsCache.delete(doc.uri.toString());
            updateStatusBar();
            return;
          }
        }
        runAnalysis(doc);
      }, ms)
    );
  };

  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((e) => schedule(e.document)),
    vscode.workspace.onDidOpenTextDocument((d) => schedule(d)),
    vscode.workspace.onDidCloseTextDocument((d) => {
      const k = d.uri.toString();
      collection.delete(d.uri);
      findingsCache.delete(k);
      contentCache.delete(k);
      debouncers.delete(k);
      updateStatusBar();
    }),
    vscode.workspace.onDidChangeWorkspaceFolders(() => {
      invalidateWorkspaceCache();
      contentCache.clear();
      findingsCache.clear();
      for (const doc of vscode.workspace.textDocuments) {
        schedule(doc);
      }
    }),
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration('sanctifier')) {
        if (e.affectsConfiguration('sanctifier.sanctifierPath')) {
          validateSanctifierPath(getConfig().get<string>('sanctifierPath') ?? '');
        }
        invalidateWorkspaceCache();
        contentCache.clear();
        findingsCache.clear();
        for (const doc of vscode.workspace.textDocuments) {
          schedule(doc);
        }
      }
    }),
    vscode.window.onDidChangeActiveTextEditor(() => {
      updateStatusBar();
    })
  );

  for (const doc of vscode.workspace.textDocuments) {
    schedule(doc);
  }

  // ── Telemetry opt-in prompt ──────────────────────────────────────────────────

  context.subscriptions.push(
    vscode.commands.registerCommand('sanctifier.toggleEnable', async () => {
      const cfg = getConfig();
      const current = cfg.get<boolean>('enable') ?? true;
      await cfg.update('enable', !current, vscode.ConfigurationTarget.Global);
      vscode.window.showInformationMessage(
        `Sanctifier is now ${!current ? 'enabled' : 'disabled'}.`
      );
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('sanctifier.showOutput', () => {
      outputChannel.show(true);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('sanctifier.analyzeWorkspace', async () => {
      const exe = getConfig().get<string>('sanctifierPath')?.trim();
      if (!exe) {
        vscode.window.showWarningMessage(
          'Set sanctifier.sanctifierPath to your sanctifier CLI binary, then run again.',
        );
        return;
      }
      if (!fs.existsSync(exe)) {
        vscode.window.showErrorMessage(
          `Sanctifier: binary not found at "${exe}". ` +
            'Update sanctifier.sanctifierPath to a valid path and try again.',
        );
        return;
      }
      const folder = vscode.workspace.workspaceFolders?.[0];
      if (!folder) {
        vscode.window.showErrorMessage('Open a folder to analyze.');
        return;
      }

      // Remote workspaces (SSH, Codespaces, WSL) use non-file URIs; the CLI
      // must run on the same machine as the source files.
      if (folder.uri.scheme !== 'file') {
        vscode.window.showErrorMessage(
          'Sanctifier workspace analysis requires a local folder. ' +
          'Remote workspaces (SSH / Codespaces / WSL) are not yet supported — ' +
          'install the CLI on the remote host and run it from the integrated terminal.'
        );
        return;
      }

      outputChannel.clear();
      outputChannel.show(true);
      outputChannel.appendLine(`[sanctifier] Scanning ${folder.uri.fsPath} …`);

      // Security (#612): warn before executing binaries outside the workspace.
      const insideWorkspace = exe.startsWith(folder.uri.fsPath);
      if (!insideWorkspace) {
        const choice = await vscode.window.showWarningMessage(
          `Sanctifier: Run a binary outside the current workspace?\n\n${exe}`,
          { modal: true },
          'Run once'
        );
        if (choice !== 'Run once') {
          return;
        }
      }

      const output = await new Promise<string | undefined>((resolve) => {
        const p = spawn(exe, ['analyze', folder.uri.fsPath, '--format', 'json'], {
          cwd: folder.uri.fsPath,
        });
        let out = '';
        let err = '';
        p.stdout.on('data', (b: Buffer) => (out += b.toString()));
        p.stderr.on('data', (b: Buffer) => (err += b.toString()));
        p.on('close', (code) => {
          if (err) {
            outputChannel.appendLine(`[sanctifier][stderr] ${err.trim()}`);
          }
          if (code !== 0 && !out) {
            outputChannel.appendLine(`[sanctifier] CLI exited with code ${code}.`);
            resolve(undefined);
          } else {
            resolve(out || undefined);
          }
        });
        p.on('error', (e) => {
          outputChannel.appendLine(`[sanctifier][error] ${e.message}`);
          resolve(undefined);
        });
      });

      if (!output) {
        vscode.window.showErrorMessage(
          'sanctifier CLI produced no output. Check sanctifierPath and the output channel.'
        );
        return;
      }
      const token = output;
      const doc = await vscode.workspace.openTextDocument({
        content: token,
        language: 'json',
      });
      await vscode.window.showTextDocument(doc, { preview: true });
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('sanctifier.exportSarif', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== 'rust') {
        vscode.window.showWarningMessage('Open a Rust file to export its findings as SARIF.');
        return;
      }
      const text = editor.document.getText();
      if (Buffer.byteLength(text, 'utf8') > MAX_SARIF_BYTES) {
        vscode.window.showErrorMessage('File is too large to analyse for SARIF export.');
        return;
      }
      const findings: EditorFinding[] = analyzeSorobanSource(text);
      const fileUri = editor.document.uri.fsPath;
      const sarifLog = findingsToSarif(findings, vscode.Uri.file(fileUri).toString());
      const content = serialiseSarif(sarifLog);

      const defaultUri = vscode.Uri.file(path.join(path.dirname(fileUri), 'sanctifier.sarif'));
      const dest = await vscode.window.showSaveDialog({
        defaultUri,
        filters: { 'SARIF files': ['sarif', 'json'], 'All files': ['*'] },
        title: 'Export Sanctifier findings as SARIF',
      });
      if (!dest) return;

      const folder = vscode.workspace.workspaceFolders?.[0];
      if (folder && !isPathWithinWorkspace(folder.uri.fsPath, dest.fsPath)) {
        vscode.window.showErrorMessage('Cannot export outside the current workspace.');
        return;
      }

      fs.writeFileSync(dest.fsPath, content, 'utf8');
      const open = await vscode.window.showInformationMessage(
        `SARIF exported: ${path.basename(dest.fsPath)}`,
        'Open'
      );
      if (open === 'Open') {
        const doc = await vscode.workspace.openTextDocument(dest);
        await vscode.window.showTextDocument(doc, { preview: true });
      }
    }),

    vscode.commands.registerCommand('sanctifier.importSarif', async () => {
      const uris = await vscode.window.showOpenDialog({
        canSelectFiles: true,
        canSelectFolders: false,
        canSelectMany: false,
        filters: { 'SARIF files': ['sarif', 'json'], 'All files': ['*'] },
        title: 'Import SARIF file',
      });
      if (!uris || uris.length === 0) return;
      const sarifPath = uris[0].fsPath;

      const folder = vscode.workspace.workspaceFolders?.[0];
      if (folder && !isPathWithinWorkspace(folder.uri.fsPath, sarifPath)) {
        vscode.window.showErrorMessage('Selected file is outside the current workspace.');
        return;
      }

      const raw = fs.readFileSync(sarifPath, 'utf8');
      const sizeCheck = validateSarifContent(raw);
      if (!sizeCheck.ok) {
        vscode.window.showErrorMessage(`Cannot import SARIF: ${sizeCheck.error}`);
        return;
      }

      let parsed: unknown;
      try {
        parsed = parseSarif(raw);
      } catch {
        vscode.window.showErrorMessage('Failed to parse SARIF file: invalid JSON.');
        return;
      }

      if (!validateSarifShape(parsed)) {
        vscode.window.showErrorMessage('File does not look like a valid SARIF 2.1.0 log.');
        return;
      }

      const countCheck = validateSarifResultCount(parsed.runs);
      if (!countCheck.ok) {
        vscode.window.showErrorMessage(`Cannot import SARIF: ${countCheck.error}`);
        return;
      }

      const findings = sarifToFindings(parsed);
      vscode.window.showInformationMessage(
        `Imported ${findings.length} finding(s) from ${path.basename(sarifPath)}.`
      );
    }),

    vscode.commands.registerCommand('sanctifier.openSarifViewer', async () => {
      const exe = getConfig().get<string>('sanctifierPath')?.trim();
      if (!exe || !fs.existsSync(exe)) {
        vscode.window.showErrorMessage(
          'Set sanctifier.sanctifierPath to the sanctifier CLI binary path before opening the full SARIF report.',
        );
        return;
      }
      const folder = vscode.workspace.workspaceFolders?.[0];
      if (!folder) {
        vscode.window.showErrorMessage('Open a workspace folder first.');
        return;
      }

      scanning = true;
      updateStatusBar();
      try {
        const output = await new Promise<string | undefined>((resolve) => {
          const p = spawn(exe, ['analyze', folder.uri.fsPath, '--format', 'sarif'], {
            cwd: folder.uri.fsPath,
          });
          let out = '';
          p.stdout.on('data', (b: Buffer) => (out += b.toString()));
          p.on('close', () => resolve(out || undefined));
          p.on('error', () => resolve(undefined));
        });

        if (!output) {
          vscode.window.showErrorMessage('Sanctifier CLI produced no SARIF output.');
          scanning = false;
          updateStatusBar();
          return;
        }

        const sarifPath = path.join(
          fs.mkdtempSync(path.join(os.tmpdir(), 'sanctifier-')),
          'results.sarif',
        );
        fs.writeFileSync(sarifPath, output, 'utf8');

        await vscode.commands.executeCommand('sarif.openLogs', [
          vscode.Uri.file(sarifPath),
        ]);
      } catch (e) {
        vscode.window.showErrorMessage(
          `Failed to open SARIF report: ${e instanceof Error ? e.message : String(e)}`,
        );
      } finally {
        scanning = false;
        updateStatusBar();
      }
    }),

    vscode.commands.registerCommand('sanctifier.suppressFinding', (args: { code: string; line: number }) => {
      vscode.window.showInformationMessage(
        `Suppress finding ${args.code} at line ${args.line} — add // sanctifier:ignore ${args.code} above the line.`,
      );
    }),
  );

  // ── Devcontainer check ──────────────────────────────────────────────────────
  void checkAndSuggestDevcontainer();

  // ── Telemetry ────────────────────────────────────────────────────────────────
  void promptTelemetryOptIn(context);

  updateStatusBar();

  // ── Public API (stable) ──────────────────────────────────────────────────────
  const api: SanctifierExtensionApi = {
    version: EXTENSION_VERSION,
    getFindings(documentUri: string): EditorFinding[] {
      return findingsCache.get(documentUri) ?? [];
    },
  };

  return api;
}

export function deactivate(): void {
  invalidateWorkspaceCache();
  void sendTelemetry();
}
