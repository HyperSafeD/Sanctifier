"use client";

import { useState, useEffect, useCallback } from "react";
import { useSearchParams } from "next/navigation";
import { AnalysisTerminal } from "../components/AnalysisTerminal";
import { FindingsList } from "../components/FindingsList";
import { Play, RotateCcw, Save, Share2, Sparkles, Terminal, Copy, Check, Trash2, X } from "lucide-react";
import type { Finding } from "../types";

const DEFAULT_CODE = `use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct HelloContract;

#[contractimpl]
impl HelloContract {
    pub fn hello(env: Env, to: Symbol) -> Symbol {
        Symbol::new(&env, "Hello")
    }
}`;


// ── Named samples for ?sample= deeplink (issue #823) ─────────────────────────
export const PLAYGROUND_SAMPLES: Record<string, { label: string; code: string }> = {
  "auth-gap": {
    label: "Auth Gap",
    code: `use soroban_sdk::{contract, contractimpl, Env, Address};

#[contract]
pub struct AuthGapContract;

#[contractimpl]
impl AuthGapContract {
    // BUG: missing caller.require_auth() — anyone can call this
    pub fn withdraw(env: Env, caller: Address, amount: i128) {
        // caller.require_auth();  <-- should be here
        let balance: i128 = env.storage().instance().get(&caller).unwrap_or(0);
        env.storage().instance().set(&caller, &(balance - amount));
    }
}`,
  },
  "overflow": {
    label: "Arithmetic Overflow",
    code: `use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct OverflowContract;

#[contractimpl]
impl OverflowContract {
    // BUG: unchecked addition can overflow
    pub fn add(env: Env, a: u32, b: u32) -> u32 {
        a + b  // use a.checked_add(b).expect("overflow") instead
    }
}`,
  },
  "unsafe-prng": {
    label: "Unsafe PRNG",
    code: `use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct PrngContract;

#[contractimpl]
impl PrngContract {
    // BUG: ledger timestamp is miner-influenceable — not safe for randomness
    pub fn random(env: Env) -> u64 {
        env.ledger().timestamp() % 100
    }
}`,
  },
  "storage-collision": {
    label: "Storage Collision",
    code: `use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

const KEY: Symbol = symbol_short!("DATA");

#[contract]
pub struct CollisionA;
#[contract]
pub struct CollisionB;

#[contractimpl]
impl CollisionA {
    // BUG: both contracts use the same storage key "DATA"
    pub fn set(env: Env, v: u32) { env.storage().instance().set(&KEY, &v); }
}

#[contractimpl]
impl CollisionB {
    pub fn get(env: Env) -> u32 { env.storage().instance().get(&KEY).unwrap_or(0) }
}`,
  },
  "reentrancy": {
    label: "Reentrancy",
    code: `use soroban_sdk::{contract, contractimpl, token, Address, Env};

#[contract]
pub struct ReentrancyContract;

#[contractimpl]
impl ReentrancyContract {
    // BUG: external call before state update — classic reentrancy
    pub fn withdraw(env: Env, caller: Address, token_addr: Address, amount: i128) {
        let balance: i128 = env.storage().instance().get(&caller).unwrap_or(0);
        assert!(balance >= amount, "insufficient");
        // State update should happen BEFORE the external call
        token::Client::new(&env, &token_addr).transfer(&env.current_contract_address(), &caller, &amount);
        env.storage().instance().set(&caller, &(balance - amount)); // too late!
    }
}`,
  },
};

const STORAGE_KEY_SNIPPETS = "playground_snippets";
const STORAGE_KEY_LAST_CODE = "playground_last_code";

interface SavedSnippet {
  id: string;
  name: string;
  code: string;
  timestamp: number;
}

export default function PlaygroundPage() {
  const [code, setCode] = useState(DEFAULT_CODE);
  const [logs, setLogs] = useState<string[]>([]);
  const [findings, setFindings] = useState<Finding[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [savedSnippets, setSavedSnippets] = useState<SavedSnippet[]>([]);
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [saveName, setSaveName] = useState("");
  const [shareToast, setShareToast] = useState<{ visible: boolean; message: string }>({
    visible: false,
    message: "",
  });
  const [copiedSnippetId, setCopiedSnippetId] = useState<string | null>(null);
  const [severityFilter, setSeverityFilter] = useState<"all" | "critical" | "high" | "medium" | "low">("all");

  // Load saved snippets from localStorage on mount

  // Load sample from ?sample= deeplink
  const searchParams = useSearchParams();
  useEffect(() => {
    const sampleId = searchParams.get("sample");
    if (sampleId && PLAYGROUND_SAMPLES[sampleId]) {
      setCode(PLAYGROUND_SAMPLES[sampleId].code);
    }
  }, [searchParams]);

  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEY_SNIPPETS);
    if (stored) {
      try {
        setSavedSnippets(JSON.parse(stored));
      } catch (e) {
        console.error("Failed to load saved snippets:", e);
      }
    }

    // Load last code if available
    const lastCode = localStorage.getItem(STORAGE_KEY_LAST_CODE);
    if (lastCode) {
      setCode(lastCode);
    }

    // Check for shared code in URL
    const params = new URLSearchParams(window.location.search);
    const sharedCode = params.get("code");
    if (sharedCode) {
      try {
        const decompressed = decompressCode(sharedCode);
        if (decompressed) {
          setCode(decompressed);
        }
      } catch (e) {
        console.error("Failed to decompress shared code:", e);
      }
    }
  }, []);

  // Save code to localStorage whenever it changes
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY_LAST_CODE, code);
  }, [code]);

  const addLog = (text: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${text}`]);
  };

  const compressCode = (text: string): string => {
    try {
      // Simple compression using base64 encoding
      const encoded = btoa(unescape(encodeURIComponent(text)));
      return encoded;
    } catch (e) {
      console.error("Compression failed:", e);
      return "";
    }
  };

  const decompressCode = (encoded: string): string | null => {
    try {
      const decoded = decodeURIComponent(escape(atob(encoded)));
      return decoded;
    } catch (e) {
      console.error("Decompression failed:", e);
      return null;
    }
  };

  const runCode = async () => {
    setIsRunning(true);
    setLogs([]);
    setFindings([]);
    addLog("Initializing Soroban environment...");
    addLog("Compiling contract to WebAssembly...");

    try {
      const response = await fetch("/api/analyze", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ source: code }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        const errorMessage =
          errorData.error || `Analysis failed with status ${response.status}`;
        addLog(`❌ Error: ${errorMessage}`);
        setIsRunning(false);
        return;
      }

      const findings: Finding[] = await response.json();
      setFindings(findings);

      addLog("✅ Build SUCCESS: contract.wasm generated");
      addLog(`📊 Analysis complete: ${findings.length} findings detected`);

      if (findings.length === 0) {
        addLog("🎉 No security issues found!");
      } else {
        const criticalCount = findings.filter((f) => f.severity === "critical").length;
        const highCount = findings.filter((f) => f.severity === "high").length;
        if (criticalCount > 0) {
          addLog(`⚠️  ${criticalCount} critical issue(s) found`);
        }
        if (highCount > 0) {
          addLog(`⚠️  ${highCount} high severity issue(s) found`);
        }
      }
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Unknown error occurred";
      addLog(`❌ Error: ${errorMessage}`);
      addLog("💡 Make sure the backend server is running");
    } finally {
      setIsRunning(false);
    }
  };

  const resetCode = () => {
    if (confirm("Reset editor to default code?")) {
      setCode(DEFAULT_CODE);
      setLogs([]);
      setFindings([]);
    }
  };

  const saveSnippet = () => {
    if (!saveName.trim()) {
      setShareToast({ visible: true, message: "Please enter a name" });
      return;
    }

    const newSnippet: SavedSnippet = {
      id: Date.now().toString(),
      name: saveName,
      code,
      timestamp: Date.now(),
    };

    const updated = [newSnippet, ...savedSnippets];
    setSavedSnippets(updated);
    localStorage.setItem(STORAGE_KEY_SNIPPETS, JSON.stringify(updated));

    setShareToast({ visible: true, message: `Saved as "${saveName}"` });
    setSaveName("");
    setShowSaveDialog(false);

    setTimeout(() => {
      setShareToast({ visible: false, message: "" });
    }, 3000);
  };

  const loadSnippet = (snippet: SavedSnippet) => {
    setCode(snippet.code);
    setLogs([]);
    setFindings([]);
  };

  const deleteSnippet = (id: string) => {
    const updated = savedSnippets.filter((s) => s.id !== id);
    setSavedSnippets(updated);
    localStorage.setItem(STORAGE_KEY_SNIPPETS, JSON.stringify(updated));
  };

  const shareSnippet = () => {
    try {
      const compressed = compressCode(code);
      if (!compressed) {
        setShareToast({ visible: true, message: "Failed to compress code" });
        return;
      }

      const shareUrl = `${window.location.origin}/playground?code=${encodeURIComponent(compressed)}`;

      navigator.clipboard.writeText(shareUrl).then(() => {
        setShareToast({ visible: true, message: "Share link copied to clipboard!" });
        setTimeout(() => {
          setShareToast({ visible: false, message: "" });
        }, 3000);
      });
    } catch (error) {
      setShareToast({ visible: true, message: "Failed to generate share link" });
      setTimeout(() => {
        setShareToast({ visible: false, message: "" });
      }, 3000);
    }
  };

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 pb-20">
      <main className="max-w-7xl mx-auto px-4 sm:px-6 py-12 space-y-8">
        {/* Header */}
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-6">
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-emerald-500 font-mono text-xs font-bold uppercase tracking-widest">
              <Sparkles size={14} />
              Alpha Feature
            </div>
            <h1 className="text-4xl font-bold tracking-tight">Soroban Playground</h1>
            <p className="text-zinc-500 max-w-xl">
              Write, compile, and test Soroban smart contracts in real-time without local setup.
            </p>
          </div>

          <div className="flex items-center gap-3">
            <button
              onClick={resetCode}
              className="p-2.5 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
              title="Reset Code"
            >
              <RotateCcw size={20} />
            </button>
            <button
              onClick={() => setShowSaveDialog(true)}
              className="p-2.5 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
              title="Save Project"
            >
              <Save size={20} />
            </button>
            <button
              onClick={shareSnippet}
              className="p-2.5 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
              title="Share Snippet"
            >
              <Share2 size={20} />
            </button>
            <button
              onClick={runCode}
              disabled={isRunning}
              className="flex items-center gap-2 px-6 py-2.5 rounded-xl bg-emerald-500 hover:bg-emerald-600 text-white font-bold transition-all shadow-lg shadow-emerald-500/20 active:scale-95 disabled:opacity-50 disabled:pointer-events-none"
            >
              <Play size={18} fill="currentColor" />
              Run Script
            </button>
          </div>
        </div>

        {/* Editor & Results Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Editor */}
          <div className="group relative flex flex-col rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 overflow-hidden shadow-xl h-[700px]">
            <div className="px-4 py-2 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50/50 dark:bg-zinc-950/50 flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-zinc-300 dark:bg-zinc-700" />
                <span className="text-xs font-mono text-zinc-500">lib.rs</span>
              </div>
              <span className="text-[10px] font-bold text-zinc-400 uppercase tracking-wider">Rust / Soroban SDK</span>
            </div>
            <textarea
              value={code}
              onChange={(e) => setCode(e.target.value)}
              spellCheck={false}
              className="flex-1 p-6 font-mono text-sm bg-transparent outline-none resize-none leading-relaxed text-zinc-700 dark:text-zinc-300 custom-scrollbar"
            />
          </div>

          {/* Results Panel */}
          <div className="flex flex-col gap-4 h-[700px] overflow-hidden">
            {/* Tabs */}
            <div className="flex gap-2 border-b border-zinc-200 dark:border-zinc-800">
              <button
                onClick={() => setSeverityFilter("all")}
                className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
                  severityFilter === "all"
                    ? "border-emerald-500 text-emerald-600 dark:text-emerald-400"
                    : "border-transparent text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
                }`}
              >
                Output
              </button>
              <button
                onClick={() => setSeverityFilter("all")}
                className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
                  severityFilter === "all" && findings.length > 0
                    ? "border-emerald-500 text-emerald-600 dark:text-emerald-400"
                    : "border-transparent text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
                }`}
              >
                Findings ({findings.length})
              </button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto custom-scrollbar">
              {findings.length > 0 ? (
                <div className="space-y-4 p-4">
                  <div className="flex gap-2 flex-wrap">
                    {(["all", "critical", "high", "medium", "low"] as const).map((severity) => (
                      <button
                        key={severity}
                        onClick={() => setSeverityFilter(severity)}
                        className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                          severityFilter === severity
                            ? "bg-emerald-500 text-white"
                            : "bg-zinc-200 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 hover:bg-zinc-300 dark:hover:bg-zinc-700"
                        }`}
                      >
                        {severity === "all" ? "All" : severity.charAt(0).toUpperCase() + severity.slice(1)}
                      </button>
                    ))}
                  </div>
                  <FindingsList findings={findings} severityFilter={severityFilter} />
                </div>
              ) : (
                <AnalysisTerminal logs={logs} isAnalyzing={isRunning} />
              )}
            </div>
          </div>
        </div>

        {/* Recent Saves */}
        {savedSnippets.length > 0 && (
          <div className="rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-6">
            <h2 className="text-lg font-bold mb-4">Recent Saves</h2>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {savedSnippets.map((snippet) => (
                <div
                  key={snippet.id}
                  className="rounded-lg border border-zinc-200 dark:border-zinc-800 p-4 hover:bg-zinc-50 dark:hover:bg-zinc-800/50 transition-colors"
                >
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <div className="flex-1 min-w-0">
                      <h3 className="font-medium truncate">{snippet.name}</h3>
                      <p className="text-xs text-zinc-500">
                        {new Date(snippet.timestamp).toLocaleDateString()}
                      </p>
                    </div>
                    <button
                      onClick={() => deleteSnippet(snippet.id)}
                      className="p-1 text-zinc-400 hover:text-red-500 transition-colors"
                      title="Delete"
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                  <button
                    onClick={() => loadSnippet(snippet)}
                    className="w-full px-3 py-2 rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 hover:bg-emerald-500/20 transition-colors text-sm font-medium"
                  >
                    Load
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}
      </main>

      {/* Save Dialog */}
      {showSaveDialog && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
          <div className="bg-white dark:bg-zinc-900 rounded-2xl p-6 max-w-sm w-full">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-bold">Save Snippet</h2>
              <button
                onClick={() => setShowSaveDialog(false)}
                className="p-1 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-lg transition-colors"
              >
                <X size={20} />
              </button>
            </div>
            <input
              type="text"
              value={saveName}
              onChange={(e) => setSaveName(e.target.value)}
              placeholder="Enter snippet name..."
              className="w-full px-4 py-2 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-emerald-500 mb-4"
              onKeyPress={(e) => {
                if (e.key === "Enter") {
                  saveSnippet();
                }
              }}
            />
            <div className="flex gap-3">
              <button
                onClick={() => setShowSaveDialog(false)}
                className="flex-1 px-4 py-2 rounded-lg border border-zinc-200 dark:border-zinc-800 text-zinc-900 dark:text-zinc-100 hover:bg-zinc-50 dark:hover:bg-zinc-800 transition-colors font-medium"
              >
                Cancel
              </button>
              <button
                onClick={saveSnippet}
                className="flex-1 px-4 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-600 text-white font-medium transition-colors"
              >
                Save
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Toast */}
      {shareToast.visible && (
        <div className="fixed bottom-4 right-4 bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 px-4 py-3 rounded-lg shadow-lg flex items-center gap-2 z-50">
          <Check size={18} />
          {shareToast.message}
        </div>
      )}

      <style jsx global>{`
        .custom-scrollbar::-webkit-scrollbar {
          width: 8px;
        }
        .custom-scrollbar::-webkit-scrollbar-track {
          background: transparent;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb {
          background: rgba(161, 161, 170, 0.2);
          border-radius: 10px;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb:hover {
          background: rgba(161, 161, 170, 0.3);
        }
      `}</style>
    </div>
  );
}
