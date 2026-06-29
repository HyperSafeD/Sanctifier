import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

const FEATURE_KEY_PREFIX = 'ghcr.io/devcontainers/features/sanctifier-cli';

export function devcontainerJsonPath(workspaceFolder: vscode.WorkspaceFolder): string {
  return path.join(workspaceFolder.uri.fsPath, '.devcontainer', 'devcontainer.json');
}

export function devcontainerExists(workspaceFolder: vscode.WorkspaceFolder): boolean {
  return fs.existsSync(devcontainerJsonPath(workspaceFolder));
}

export function hasSanctifierFeature(config: Record<string, unknown>): boolean {
  const features = config.features as Record<string, unknown> | undefined;
  if (!features) return false;
  return Object.keys(features).some((k) => k.includes('sanctifier-cli'));
}

export async function suggestAddToDevcontainer(
  devcontainerPath: string,
): Promise<void> {
  const choice = await vscode.window.showInformationMessage(
    'This project has a .devcontainer but sanctifier-cli is not configured as a feature. Add it for consistent security scanning in Codespaces?',
    'Add to devcontainer',
  );
  if (choice !== 'Add to devcontainer') return;

  try {
    const raw = fs.readFileSync(devcontainerPath, 'utf8');
    const config = JSON.parse(raw) as Record<string, unknown>;
    if (!config.features || typeof config.features !== 'object') {
      config.features = {};
    }
    const features = config.features as Record<string, unknown>;
    features[`${FEATURE_KEY_PREFIX}:1`] = {};

    const updated = JSON.stringify(config, null, 2) + '\n';
    fs.writeFileSync(devcontainerPath, updated, 'utf8');

    vscode.window.showInformationMessage(
      'sanctifier-cli feature added to devcontainer.json. Rebuild the container to apply.',
    );
  } catch (e) {
    vscode.window.showErrorMessage(
      `Failed to update devcontainer.json: ${e instanceof Error ? e.message : String(e)}`,
    );
  }
}

export async function checkAndSuggestDevcontainer(): Promise<void> {
  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) return;

  const dcPath = devcontainerJsonPath(folder);
  if (!fs.existsSync(dcPath)) return;

  try {
    const raw = fs.readFileSync(dcPath, 'utf8');
    const config = JSON.parse(raw) as Record<string, unknown>;
    if (!hasSanctifierFeature(config)) {
      await suggestAddToDevcontainer(dcPath);
    }
  } catch {
    // Devcontainer JSON parse error — not actionable, skip silently.
  }
}
