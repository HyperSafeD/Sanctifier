import * as vscode from 'vscode';

const folderCache = new Map<string, boolean>();

export function invalidateWorkspaceCache(): void {
  folderCache.clear();
}

export async function folderLooksLikeSorobanProject(
  folder: vscode.WorkspaceFolder
): Promise<boolean> {
  const key = folder.uri.toString();
  const cached = folderCache.get(key);
  if (cached !== undefined) {
    return cached;
  }
  const pattern = new vscode.RelativePattern(folder, '**/Cargo.toml');
  const files = await vscode.workspace.findFiles(pattern, '**/target/**', 40);
  for (const uri of files) {
    try {
      const doc = await vscode.workspace.openTextDocument(uri);
      if (/soroban-sdk|soroban_sdk/.test(doc.getText())) {
        folderCache.set(key, true);
        return true;
      }
    } catch {
      /* skip unreadable files */
    }
  }
  folderCache.set(key, false);
  return false;
}
