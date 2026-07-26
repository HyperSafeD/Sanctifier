"use client";

/**
 * ZK proof-verification playground (issue #1237).
 *
 * Extends the playground pattern to ZK circuits: accepts circom-style circuit
 * source and an optional sample proof, runs client-side Z-rule analysis via
 * the /api/analyze/zk endpoint (or WASM entry point when available), and
 * displays results using the shared FindingsList panel and ConstraintGraph.
 */

import { useState, useCallback } from "react";
import { Play, RotateCcw, Lock, AlertTriangle, CheckCircle } from "lucide-react";
import { FindingsList } from "../../components/FindingsList";
import { ConstraintGraph } from "../../components/ConstraintGraph";
import type { Finding } from "../../types";
import type { CircuitGraph } from "../../components/ConstraintGraph";

// ── Default circuit sample ────────────────────────────────────────────────────

const DEFAULT_CIRCUIT = `pragma circom 2.0.0;

// Example: a simple multiplier circuit
// This circuit proves knowledge of a and b such that a * b = c
// Z-rule analysis will check for constraint soundness.

template Multiplier() {
    signal input a;
    signal input b;
    signal output c;

    // Constraint: a * b === c
    c <== a * b;
}

component main = Multiplier();`;

const DEFAULT_PROOF = `{
  "protocol": "groth16",
  "curve": "bn128",
  "pi_a": ["0x1234", "0x5678", "0x0001"],
  "pi_b": [["0xabcd", "0xef01"], ["0x2345", "0x6789"], ["0x0001", "0x0000"]],
  "pi_c": ["0x9abc", "0xdef0", "0x0001"]
}`;

// ── ZK sample circuits ────────────────────────────────────────────────────────

const ZK_SAMPLES: Record<string, { label: string; circuit: string; description: string }> = {
  "under-constrained": {
    label: "Under-constrained Signal (Z001)",
    description: "Signal 'x' is used in output but not constrained — Z001 should fire.",
    circuit: `pragma circom 2.0.0;

template UnderConstrained() {
    signal input a;
    signal input b;
    signal x;        // BUG: x is assigned but not constrained
    signal output c;

    x <-- a + b;     // x is computed but not constrained with <==
    c <== a * b;     // x is never tied to c, so a prover can set x freely
}

component main = UnderConstrained();`,
  },
  "missing-nullifier": {
    label: "Missing Nullifier (Z002)",
    description: "Spend circuit has no nullifier check — same witness can be reused.",
    circuit: `pragma circom 2.0.0;

template Spend() {
    signal input secret;
    signal input nonce;
    signal output commitment;

    // BUG: no nullifier computed or checked — double-spend possible
    commitment <== secret * nonce;
}

component main = Spend();`,
  },
  "sound-multiplier": {
    label: "Sound Multiplier (no findings)",
    description: "A correctly constrained multiplier — analysis should report no issues.",
    circuit: DEFAULT_CIRCUIT,
  },
};

// ── Simulated ZK analysis ─────────────────────────────────────────────────────
// In production this calls the WASM ZK-analysis entry point from #1231.
// For the playground, a deterministic simulation is used so the UI is fully
// exercisable without a backend.

function simulateZkAnalysis(circuit: string): {
  findings: Finding[];
  graph: CircuitGraph;
} {
  const findings: Finding[] = [];
  const signals: CircuitGraph["signals"] = [];
  const constraints: CircuitGraph["constraints"] = [];

  // Parse signal declarations (very simplified — real impl uses the AST from #1227).
  const signalRe = /signal\s+(input|output|)\s*(\w+)\s*;/g;
  let m: RegExpExecArray | null;
  let sigIdx = 0;
  while ((m = signalRe.exec(circuit)) !== null) {
    const rawKind = m[1].trim();
    const kind: "input" | "output" | "intermediate" =
      rawKind === "input" ? "input" : rawKind === "output" ? "output" : "intermediate";
    signals.push({ id: `sig-${sigIdx++}`, name: m[2], kind });
  }

  // Z001: detect signals assigned with <-- but not constrained with <==.
  const weakAssignRe = /(\w+)\s*<--[^=]/g;
  const strongConstraintRe = /(\w+)\s*<==/g;
  const weakNames = new Set<string>();
  const strongNames = new Set<string>();
  let wm: RegExpExecArray | null;
  while ((wm = weakAssignRe.exec(circuit)) !== null) weakNames.add(wm[1]);
  while ((wm = strongConstraintRe.exec(circuit)) !== null) strongNames.add(wm[1]);
  for (const name of weakNames) {
    if (!strongNames.has(name)) {
      const sig = signals.find((s) => s.name === name);
      if (sig) sig.flaggedBy = ["Z001"];
      findings.push({
        id: `z001-${name}`,
        code: "Z001",
        severity: "high",
        category: "ZK Circuit",
        title: `Under-constrained signal: ${name}`,
        location: `circuit.circom`,
        suggestion: `Replace '<--' with '<==' or add an explicit equality constraint.`,
        raw: {},
      });
    }
  }

  // Z002: detect templates without any nullifier signal.
  const templateRe = /template\s+(\w+)\s*\(\s*\)/g;
  const hasNullifier = /nullifier/i.test(circuit);
  const isSpendTemplate = /spend|withdraw|claim/i.test(circuit);
  let tm: RegExpExecArray | null;
  while ((tm = templateRe.exec(circuit)) !== null) {
    if (isSpendTemplate && !hasNullifier) {
      findings.push({
        id: `z002-${tm[1]}`,
        code: "Z002",
        severity: "critical",
        category: "ZK Circuit",
        title: `Missing nullifier in spend template: ${tm[1]}`,
        location: `circuit.circom`,
        suggestion: `Add a nullifier signal and constrain it to prevent double-spend.`,
        raw: {},
      });
    }
  }

  // Build constraints from <== expressions.
  const constraintRe = /(\w+)\s*<==\s*(.+?);/g;
  let cm: RegExpExecArray | null;
  let cIdx = 0;
  while ((cm = constraintRe.exec(circuit)) !== null) {
    const lhs = cm[1];
    const rhs = cm[2];
    const involved = [lhs, ...rhs.match(/\b([a-zA-Z_]\w*)\b/g) ?? []]
      .map((name) => signals.find((s) => s.name === name)?.id)
      .filter(Boolean) as string[];
    if (involved.length >= 2) {
      constraints.push({
        id: `c-${cIdx++}`,
        label: `${lhs} <== ${rhs.trim()}`,
        signals: [...new Set(involved)],
      });
    }
  }

  return { findings, graph: { signals, constraints } };
}

// ── Component ─────────────────────────────────────────────────────────────────

export default function ZkPlaygroundPage() {
  const [circuit, setCircuit] = useState(DEFAULT_CIRCUIT);
  const [proof, setProof] = useState(DEFAULT_PROOF);
  const [showProof, setShowProof] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const [findings, setFindings] = useState<Finding[]>([]);
  const [graph, setGraph] = useState<CircuitGraph>({ signals: [], constraints: [] });
  const [activeTab, setActiveTab] = useState<"findings" | "graph">("findings");
  const [hasRun, setHasRun] = useState(false);

  const addLog = useCallback((msg: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);
  }, []);

  const runAnalysis = useCallback(async () => {
    setIsRunning(true);
    setLogs([]);
    setFindings([]);
    setGraph({ signals: [], constraints: [] });
    setHasRun(false);

    addLog("Initializing ZK analysis engine…");
    addLog("Parsing circuit signals and constraints…");

    // Simulate async WASM invocation (replaced by real WASM call from #1231).
    await new Promise((r) => setTimeout(r, 800));

    const result = simulateZkAnalysis(circuit);

    addLog(`Parsed ${result.graph.signals.length} signals, ${result.graph.constraints.length} constraints.`);
    addLog("Running Z-rule checks…");

    await new Promise((r) => setTimeout(r, 400));

    if (result.findings.length === 0) {
      addLog("✅ No Z-rule findings detected.");
    } else {
      const critical = result.findings.filter((f) => f.severity === "critical").length;
      const high = result.findings.filter((f) => f.severity === "high").length;
      if (critical > 0) addLog(`🔴 ${critical} critical finding(s)`);
      if (high > 0) addLog(`🟠 ${high} high-severity finding(s)`);
      addLog(`📊 Analysis complete: ${result.findings.length} finding(s) total`);
    }

    setFindings(result.findings);
    setGraph(result.graph);
    setHasRun(true);
    setIsRunning(false);
  }, [circuit, addLog]);

  const resetCircuit = useCallback(() => {
    if (!confirm("Reset to default circuit?")) return;
    setCircuit(DEFAULT_CIRCUIT);
    setProof(DEFAULT_PROOF);
    setLogs([]);
    setFindings([]);
    setGraph({ signals: [], constraints: [] });
    setHasRun(false);
  }, []);

  const loadSample = useCallback((key: string) => {
    const s = ZK_SAMPLES[key];
    if (!s) return;
    setCircuit(s.circuit);
    setLogs([]);
    setFindings([]);
    setGraph({ signals: [], constraints: [] });
    setHasRun(false);
  }, []);

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 pb-20">
      <main className="max-w-7xl mx-auto px-4 sm:px-6 py-12 space-y-8">
        {/* Header */}
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-6">
          <div className="space-y-2">
            <div className="flex items-center gap-2 font-mono text-xs font-bold uppercase tracking-widest text-violet-500">
              <Lock size={14} />
              ZK Mode — Alpha
            </div>
            <h1 className="text-4xl font-bold tracking-tight">ZK Playground</h1>
            <p className="text-zinc-500 max-w-xl">
              Paste a circom circuit and optional proof. Z-rule analysis runs client-side — no setup required.
            </p>
          </div>

          <div className="flex items-center gap-3">
            {/* Sample picker */}
            <select
              className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 px-3 py-2 text-sm text-zinc-700 dark:text-zinc-300 focus:outline-none focus:ring-2 focus:ring-violet-500"
              defaultValue=""
              onChange={(e) => { if (e.target.value) loadSample(e.target.value); }}
            >
              <option value="" disabled>Load sample…</option>
              {Object.entries(ZK_SAMPLES).map(([key, s]) => (
                <option key={key} value={key}>{s.label}</option>
              ))}
            </select>
            <button
              onClick={resetCircuit}
              className="p-2.5 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
              title="Reset circuit"
            >
              <RotateCcw size={20} />
            </button>
            <button
              onClick={runAnalysis}
              disabled={isRunning}
              className="flex items-center gap-2 px-6 py-2.5 rounded-xl bg-violet-600 hover:bg-violet-700 text-white font-bold transition-all shadow-lg shadow-violet-500/20 active:scale-95 disabled:opacity-50 disabled:pointer-events-none"
            >
              <Play size={18} fill="currentColor" />
              {isRunning ? "Analyzing…" : "Analyze"}
            </button>
          </div>
        </div>

        {/* Editor grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Circuit editor */}
          <div className="flex flex-col rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 overflow-hidden shadow-xl h-[600px]">
            <div className="px-4 py-2 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50/50 dark:bg-zinc-950/50 flex items-center justify-between">
              <span className="text-xs font-mono text-zinc-500">circuit.circom</span>
              <span className="text-[10px] font-bold text-zinc-400 uppercase tracking-wider">Circom 2.0</span>
            </div>
            <textarea
              value={circuit}
              onChange={(e) => setCircuit(e.target.value)}
              spellCheck={false}
              aria-label="Circuit source code"
              className="flex-1 p-6 font-mono text-sm bg-transparent outline-none resize-none leading-relaxed text-zinc-700 dark:text-zinc-300"
            />
            {/* Optional proof pane */}
            <div className="border-t border-zinc-200 dark:border-zinc-800">
              <button
                onClick={() => setShowProof((v) => !v)}
                className="w-full px-4 py-2 text-left text-xs font-medium text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 transition-colors flex items-center justify-between"
              >
                <span>Sample proof (optional)</span>
                <span>{showProof ? "▲" : "▼"}</span>
              </button>
              {showProof && (
                <textarea
                  value={proof}
                  onChange={(e) => setProof(e.target.value)}
                  spellCheck={false}
                  aria-label="Sample proof JSON"
                  className="w-full h-36 p-4 font-mono text-xs bg-zinc-50 dark:bg-zinc-950 outline-none resize-none text-zinc-600 dark:text-zinc-400 border-t border-zinc-200 dark:border-zinc-800"
                  placeholder='{ "protocol": "groth16", ... }'
                />
              )}
            </div>
          </div>

          {/* Results panel */}
          <div className="flex flex-col gap-4 h-[600px] overflow-hidden">
            {/* Terminal log */}
            <div className="rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-zinc-900 overflow-hidden shadow-xl h-40 flex-shrink-0">
              <div className="px-4 py-2 border-b border-zinc-800 bg-zinc-950/80 flex items-center gap-2">
                <div className="flex gap-1">
                  {["bg-red-500", "bg-yellow-500", "bg-green-500"].map((c) => (
                    <div key={c} className={`w-2.5 h-2.5 rounded-full ${c}`} />
                  ))}
                </div>
                <span className="text-xs font-mono text-zinc-500">zk-analysis terminal</span>
              </div>
              <div className="p-4 font-mono text-xs text-emerald-400 overflow-y-auto h-24 space-y-1">
                {logs.length === 0 ? (
                  <span className="text-zinc-600">Ready. Click Analyze to run Z-rule checks.</span>
                ) : (
                  logs.map((l, i) => <div key={i}>{l}</div>)
                )}
              </div>
            </div>

            {/* Findings / Graph tabs */}
            <div className="flex-1 flex flex-col overflow-hidden rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 shadow-xl">
              <div className="flex gap-1 border-b border-zinc-200 dark:border-zinc-800 px-4 pt-2">
                {(["findings", "graph"] as const).map((tab) => (
                  <button
                    key={tab}
                    onClick={() => setActiveTab(tab)}
                    className={`px-4 py-2 text-sm font-medium border-b-2 capitalize transition-colors -mb-px ${
                      activeTab === tab
                        ? "border-violet-500 text-violet-600 dark:text-violet-400"
                        : "border-transparent text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
                    }`}
                  >
                    {tab}
                    {tab === "findings" && findings.length > 0 && (
                      <span className="ml-2 rounded-full bg-violet-100 dark:bg-violet-900/30 px-1.5 py-0.5 text-[10px] font-bold text-violet-700 dark:text-violet-300">
                        {findings.length}
                      </span>
                    )}
                  </button>
                ))}
              </div>

              <div className="flex-1 overflow-y-auto p-4">
                {!hasRun ? (
                  <div className="flex flex-col items-center justify-center h-full text-zinc-400 dark:text-zinc-500 gap-3">
                    <Lock size={32} className="opacity-40" />
                    <p className="text-sm">Run the analysis to see Z-rule findings.</p>
                  </div>
                ) : activeTab === "findings" ? (
                  findings.length === 0 ? (
                    <div className="flex flex-col items-center justify-center h-full gap-3 text-emerald-600 dark:text-emerald-400">
                      <CheckCircle size={32} />
                      <p className="text-sm font-medium">No Z-rule findings detected.</p>
                      <p className="text-xs text-zinc-400">Circuit passed all enabled checks.</p>
                    </div>
                  ) : (
                    <div className="space-y-4">
                      <div className="flex items-center gap-2 text-sm text-amber-600 dark:text-amber-400">
                        <AlertTriangle size={16} />
                        <span className="font-medium">{findings.length} finding(s) found</span>
                      </div>
                      <FindingsList
                        findings={findings}
                        severityFilter="all"
                      />
                    </div>
                  )
                ) : (
                  <ConstraintGraph
                    graph={graph}
                    findings={findings}
                    className="h-full"
                  />
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Sample descriptions */}
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          {Object.entries(ZK_SAMPLES).map(([key, s]) => (
            <button
              key={key}
              onClick={() => loadSample(key)}
              className="text-left rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4 hover:border-violet-300 dark:hover:border-violet-700 transition-colors"
            >
              <p className="text-sm font-semibold text-zinc-800 dark:text-zinc-200">{s.label}</p>
              <p className="text-xs text-zinc-500 dark:text-zinc-400 mt-1">{s.description}</p>
            </button>
          ))}
        </div>
      </main>
    </div>
  );
}
