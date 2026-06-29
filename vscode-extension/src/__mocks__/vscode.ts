// Minimal VS Code API stub for unit tests (jest environment — no real VS Code available).
export const window = {
  showWarningMessage: jest.fn(),
  showErrorMessage: jest.fn(),
  showInformationMessage: jest.fn(),
  showSaveDialog: jest.fn(),
  showOpenDialog: jest.fn(),
  activeTextEditor: undefined,
};

export const workspace = {
  getConfiguration: jest.fn(() => ({
    get: jest.fn(),
  })),
  workspaceFolders: undefined,
  findFiles: jest.fn(() => Promise.resolve([])),
  openTextDocument: jest.fn(),
  textDocuments: [],
  onDidChangeTextDocument: jest.fn(() => ({ dispose: jest.fn() })),
  onDidOpenTextDocument: jest.fn(() => ({ dispose: jest.fn() })),
  onDidCloseTextDocument: jest.fn(() => ({ dispose: jest.fn() })),
  onDidChangeConfiguration: jest.fn(() => ({ dispose: jest.fn() })),
};

export const languages = {
  createDiagnosticCollection: jest.fn(() => ({
    set: jest.fn(),
    delete: jest.fn(),
    clear: jest.fn(),
    dispose: jest.fn(),
  })),
  registerHoverProvider: jest.fn(() => ({ dispose: jest.fn() })),
  registerCodeActionsProvider: jest.fn(() => ({ dispose: jest.fn() })),
};

export const commands = {
  registerCommand: jest.fn(() => ({ dispose: jest.fn() })),
};

export class Uri {
  static file(p: string): Uri { return new Uri(p); }
  static parse(s: string): Uri { return new Uri(s); }
  constructor(public fsPath: string) {}
  toString() { return `file://${this.fsPath}`; }
}

export enum DiagnosticSeverity {
  Error = 0,
  Warning = 1,
  Information = 2,
  Hint = 3,
}

export class Range {
  readonly start: Position;
  readonly end: Position;
  constructor(startLine: number, startCharacter: number, endLine: number, endCharacter: number) {
    this.start = new Position(startLine, startCharacter);
    this.end = new Position(endLine, endCharacter);
  }
}

export class Position {
  constructor(public line: number, public character: number) {}
}

export class Diagnostic {
  public code?: string | number | { value: string | number; target: Uri };
  public source?: string;
  constructor(
    public range: Range,
    public message: string,
    public severity: DiagnosticSeverity
  ) {}
}

export class WorkspaceEdit {
  readonly _inserts: Array<{ uri: Uri; position: Position; text: string }> = [];
  readonly _replaces: Array<{ uri: Uri; range: Range; text: string }> = [];

  insert(uri: Uri, position: Position, text: string): void {
    this._inserts.push({ uri, position, text });
  }

  replace(uri: Uri, range: Range, text: string): void {
    this._replaces.push({ uri, range, text });
  }
}

export const CodeActionKind = {
  QuickFix: { value: 'quickfix' },
  RefactorRewrite: { value: 'refactor.rewrite' },
  Empty: { value: '' },
};

export class CodeAction {
  public diagnostics?: Diagnostic[];
  public edit?: WorkspaceEdit;
  public isPreferred?: boolean;
  constructor(
    public title: string,
    public kind?: { value: string },
  ) {}
}
