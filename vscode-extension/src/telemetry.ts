import * as vscode from 'vscode';

interface TelemetryPayload {
  scan_count: number;
  finding_count_by_rule: Record<string, number>;
  extension_version: string;
  vscode_version: string;
}

const TELEMETRY_ENDPOINT = 'https://telemetry.hypersafed.com/api/v1/ingest';

const EXTENSION_VERSION: string = (() => {
  try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    return (require('../package.json') as { version: string }).version;
  } catch {
    return '0.0.0';
  }
})();

let scanCount = 0;
const findingCountByRule: Record<string, number> = {};

export function recordScan(findingsByRule: Record<string, number>): void {
  scanCount++;
  for (const [rule, count] of Object.entries(findingsByRule)) {
    findingCountByRule[rule] = (findingCountByRule[rule] ?? 0) + count;
  }
}

export function buildPayload(): TelemetryPayload {
  return {
    scan_count: scanCount,
    finding_count_by_rule: { ...findingCountByRule },
    extension_version: EXTENSION_VERSION,
    vscode_version: vscode.version,
  };
}

export async function sendTelemetry(): Promise<void> {
  const cfg = vscode.workspace.getConfiguration('sanctifier');
  if (!cfg.get<boolean>('telemetry.enabled')) {
    return;
  }
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5000);
    await fetch(TELEMETRY_ENDPOINT, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(buildPayload()),
      signal: controller.signal,
    });
    clearTimeout(timeout);
  } catch {
    /* silently drop telemetry errors */
  }
}

export function resetTelemetry(): void {
  scanCount = 0;
  for (const key of Object.keys(findingCountByRule)) {
    delete findingCountByRule[key];
  }
}

export async function promptTelemetryOptIn(context: vscode.ExtensionContext): Promise<void> {
  const key = 'sanctifier.telemetry.firstRunPrompted';
  if (context.globalState.get<boolean>(key)) {
    return;
  }
  await context.globalState.update(key, true);

  const cfg = vscode.workspace.getConfiguration('sanctifier');
  const alreadySet = cfg.inspect<boolean>('telemetry.enabled');
  if (alreadySet?.globalValue !== undefined) {
    return;
  }

  const choice = await vscode.window.showInformationMessage(
    'Sanctifier: Help us improve by sharing anonymous usage telemetry? No source code or file paths are ever sent.',
    { modal: false },
    'Yes, enable telemetry',
    'No, thanks',
    'Read privacy policy',
  );

  if (choice === 'Yes, enable telemetry') {
    await cfg.update('telemetry.enabled', true, vscode.ConfigurationTarget.Global);
  } else if (choice === 'No, thanks') {
    await cfg.update('telemetry.enabled', false, vscode.ConfigurationTarget.Global);
  } else if (choice === 'Read privacy policy') {
    await vscode.env.openExternal(
      vscode.Uri.parse('https://github.com/HyperSafeD/Sanctifier/blob/main/vscode-extension/PRIVACY.md'),
    );
    const followUp = await vscode.window.showInformationMessage(
      'Sanctifier: Enable anonymous usage telemetry?',
      'Yes',
      'No',
    );
    if (followUp === 'Yes') {
      await cfg.update('telemetry.enabled', true, vscode.ConfigurationTarget.Global);
    } else if (followUp === 'No') {
      await cfg.update('telemetry.enabled', false, vscode.ConfigurationTarget.Global);
    }
  }
}
