"use client";

import { useState } from "react";
import { AnalysisTerminal } from "../components/AnalysisTerminal";
import { Play, RotateCcw, Save, Share2, Sparkles, Terminal } from "lucide-react";

const DEFAULT_CODE = `use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct HelloContract;

#[contractimpl]
impl HelloContract {
    pub fn hello(env: Env, to: Symbol) -> Symbol {
        Symbol::new(&env, "Hello")
    }
}`;

export default function PlaygroundPage() {
  const [code, setCode] = useState(DEFAULT_CODE);
  const [logs, setLogs] = useState<string[]>([]);
  const [isRunning, setIsRunning] = useState(false);

  const addLog = (text: string) => {
    setLogs((prev) => [...prev, \`[\${new Date().toLocaleTimeString()}] \${text}\`]);
  };

  const runCode = async () => {
    setIsRunning(true);
    setLogs([]);
    addLog("Initializing Soroban environment...");
    addLog("Compiling contract to WebAssembly...");
    
    // Simulate compilation and execution
    setTimeout(() => {
      addLog("Build SUCCESS: contract.wasm generated (2.4 KB)");
      addLog("Deploying to local sandbox...");
      setTimeout(() => {
        addLog("INVOKE: HelloContract::hello(to: 'World')");
        addLog("RESULT: 'Hello'");
        addLog("Resource Usage: CPU: 1423, Memory: 421 bytes");
        setIsRunning(false);
      }, 1000);
    }, 1500);
  };

  const resetCode = () => {
    if (confirm("Reset editor to default code?")) {
      setCode(DEFAULT_CODE);
      setLogs([]);
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
              className="p-2.5 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
              title="Save Project"
            >
              <Save size={20} />
            </button>
            <button
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

        {/* Editor & Terminal Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 h-[700px]">
          {/* Editor Placeholder */}
          <div className="group relative flex flex-col rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 overflow-hidden shadow-xl">
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

          {/* Terminal */}
          <div className="flex flex-col gap-4">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-bold flex items-center gap-2 text-zinc-500">
                <Terminal size={16} />
                Cloud Execution Output
              </h2>
            </div>
            <AnalysisTerminal logs={logs} isAnalyzing={isRunning} />
          </div>
        </div>
      </main>

      <style jsx global>{\`
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
      \`}</style>
    </div>
  );
}
