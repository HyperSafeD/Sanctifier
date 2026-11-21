/**
 * Unit tests for workspace-trust guard behaviour.
 *
 * These tests use a lightweight stub of the VS Code API surface that the
 * trust-guard logic actually touches:
 *   - vscode.workspace.isTrusted
 *   - vscode.workspace.onDidGrantWorkspaceTrust
 *   - DiagnosticCollection.clear / delete
 *
 * They do NOT activate the full extension; they verify the pure logic that
 * schedule() and runAnalysis() use to decide whether to proceed.
 */

import * as assert from 'node:assert/strict';
import { describe, it, beforeEach } from 'node:test';

// ---------------------------------------------------------------------------
// Minimal stubs
// ---------------------------------------------------------------------------

type Listener<T> = (value: T) => void;

function makeWorkspaceStub(trusted: boolean) {
  const listeners: Array<Listener<void>> = [];
  const stub = {
    isTrusted: trusted,
    onDidGrantWorkspaceTrust(cb: Listener<void>) {
      listeners.push(cb);
      return { dispose: () => void 0 };
    },
    _grantTrust() {
      stub.isTrusted = true;
      for (const l of listeners) l();
    },
  };
  return stub;
}

interface DiagEntry { uri: string; deleted: boolean }

function makeDiagCollection() {
  const entries = new Map<string, DiagEntry>();
  return {
    set(uri: { toString(): string }) {
      entries.set(uri.toString(), { uri: uri.toString(), deleted: false });
    },
    delete(uri: { toString(): string }) {
      const key = uri.toString();
      entries.set(key, { uri: key, deleted: true });
    },
    clear() {
      for (const key of entries.keys()) {
        entries.set(key, { uri: key, deleted: true });
      }
    },
    isDeleted(uriStr: string) {
      return entries.get(uriStr)?.deleted ?? false;
    },
    size() {
      return [...entries.values()].filter((e) => !e.deleted).length;
    },
  };
}

// ---------------------------------------------------------------------------
// The logic under test, extracted to be testable without vscode module
// ---------------------------------------------------------------------------

function makeAnalysisGuard(workspace: ReturnType<typeof makeWorkspaceStub>) {
  const findingsCache = new Map<string, string[]>();
  const contentCache = new Map<string, string>();
  const debouncers = new Map<string, ReturnType<typeof setTimeout>>();
  const collection = makeDiagCollection();
  let scheduledDocs: string[] = [];

  function cancelAll() {
    for (const [key, timer] of debouncers) {
      clearTimeout(timer);
      debouncers.delete(key);
    }
    collection.clear();
    findingsCache.clear();
    contentCache.clear();
  }

  function schedule(uriStr: string) {
    if (!workspace.isTrusted) {
      const prev = debouncers.get(uriStr);
      if (prev) { clearTimeout(prev); debouncers.delete(uriStr); }
      collection.delete({ toString: () => uriStr });
      findingsCache.delete(uriStr);
      return false; // did NOT schedule
    }
    scheduledDocs.push(uriStr);
    return true; // did schedule
  }

  function runAnalysis(uriStr: string): boolean {
    if (!workspace.isTrusted) return false;
    findingsCache.set(uriStr, ['finding-1']);
    return true;
  }

  workspace.onDidGrantWorkspaceTrust(() => {
    findingsCache.clear();
    contentCache.clear();
    for (const uri of scheduledDocs) schedule(uri);
  });

  if (!workspace.isTrusted) cancelAll();

  return { schedule, runAnalysis, findingsCache, collection, debouncers, cancelAll, scheduledDocs };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('workspace-trust guard', () => {
  describe('when workspace opens untrusted', () => {
    let ws: ReturnType<typeof makeWorkspaceStub>;
    let guard: ReturnType<typeof makeAnalysisGuard>;

    beforeEach(() => {
      ws = makeWorkspaceStub(false);
      guard = makeAnalysisGuard(ws);
    });

    it('schedule() returns false and does not queue analysis', () => {
      const result = guard.schedule('file:///src/lib.rs');
      assert.equal(result, false);
    });

    it('runAnalysis() returns false and does not populate findingsCache', () => {
      const result = guard.runAnalysis('file:///src/lib.rs');
      assert.equal(result, false);
      assert.equal(guard.findingsCache.size, 0);
    });

    it('existing diagnostics are cleared on activation in restricted mode', () => {
      // Simulate a doc that was analysed before trust was revoked
      guard.collection.set({ toString: () => 'file:///src/old.rs' });
      // cancelAll was already called in makeAnalysisGuard constructor — the
      // collection.clear() path is exercised; the entry should be marked deleted.
      assert.ok(guard.collection.isDeleted('file:///src/old.rs'));
    });

    it('pending debouncers are cancelled when schedule() is called untrusted', () => {
      // Plant a fake debouncer
      const timer = setTimeout(() => void 0, 10_000);
      guard.debouncers.set('file:///src/lib.rs', timer);

      guard.schedule('file:///src/lib.rs');
      assert.equal(guard.debouncers.has('file:///src/lib.rs'), false);
      clearTimeout(timer); // cleanup in case clearTimeout inside wasn't called
    });
  });

  describe('when trust is granted after opening untrusted', () => {
    let ws: ReturnType<typeof makeWorkspaceStub>;
    let guard: ReturnType<typeof makeAnalysisGuard>;

    beforeEach(() => {
      ws = makeWorkspaceStub(false);
      guard = makeAnalysisGuard(ws);
      // Pre-populate caches to verify they are cleared on trust grant
      guard.findingsCache.set('file:///src/lib.rs', ['stale']);
    });

    it('onDidGrantWorkspaceTrust listener clears the findings cache', () => {
      ws._grantTrust();
      assert.equal(guard.findingsCache.size, 0);
    });

    it('schedule() returns true after trust is granted', () => {
      ws._grantTrust();
      const result = guard.schedule('file:///src/lib.rs');
      assert.equal(result, true);
    });

    it('runAnalysis() populates findings after trust is granted', () => {
      ws._grantTrust();
      guard.runAnalysis('file:///src/lib.rs');
      assert.ok(guard.findingsCache.has('file:///src/lib.rs'));
    });
  });

  describe('when workspace opens trusted', () => {
    it('schedule() returns true immediately', () => {
      const ws = makeWorkspaceStub(true);
      const guard = makeAnalysisGuard(ws);
      assert.equal(guard.schedule('file:///src/lib.rs'), true);
    });

    it('runAnalysis() populates findings', () => {
      const ws = makeWorkspaceStub(true);
      const guard = makeAnalysisGuard(ws);
      guard.runAnalysis('file:///src/lib.rs');
      assert.equal(guard.findingsCache.size, 1);
    });
  });
});
