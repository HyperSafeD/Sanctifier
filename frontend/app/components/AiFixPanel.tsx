"use client";

import { X, Sparkles, Loader2, Check, Copy } from "lucide-react";
import { useState, useEffect } from "react";
import type { Finding } from "../types";
import { CodeSnippet } from "./CodeSnippet";

interface AiFixPanelProps {
  finding: Finding | null;
  onClose: () => void;
}

export function AiFixPanel({ finding, onClose }: AiFixPanelProps) {
  const [explanation, setExplanation] = useState<string | null>(null);
  const [fixCode, setFixCode] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (finding) {
      fetchExplanation();
    }
  }, [finding]);

  const fetchExplanation = async () => {
    if (!finding) return;
    setIsLoading(true);
    setExplanation(null);
    setFixCode(null);

    try {
      const response = await fetch("/api/ai/explain", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ finding }),
      });
      const data = await response.json();
      setExplanation(data.explanation);
      setFixCode(data.fixCode);
    } catch (err) {
      setExplanation("Failed to generate AI explanation. Please try again later.");
    } finally {
      setIsLoading(false);
    }
  };

  const copyCode = () => {
    if (fixCode) {
      navigator.clipboard.writeText(fixCode);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  if (!finding) return null;

  return (
    <div className="fixed inset-y-0 right-0 w-full sm:w-[500px] z-[100] bg-white dark:bg-zinc-900 border-l border-zinc-200 dark:border-zinc-800 shadow-2xl transition-transform duration-300 transform translate-x-0 animate-in slide-in-from-right-full">
      <div className="flex flex-col h-full">
        {/* Header */}
        <div className="p-6 border-b border-zinc-200 dark:border-zinc-800 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-emerald-500/10 text-emerald-500 flex items-center justify-center">
              <Sparkles size={20} />
            </div>
            <div>
              <h2 className="font-bold text-lg">AI Sanctity Guard</h2>
              <p className="text-xs text-zinc-500">Intelligent Fix Recommendations</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-lg transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-8">
          <section className="space-y-3">
            <span className="text-[10px] font-bold uppercase tracking-widest text-emerald-500">Target Finding</span>
            <div className="p-4 rounded-xl bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800">
              <h3 className="font-bold text-sm tracking-tight">{finding.title}</h3>
              <p className="text-xs text-zinc-500 mt-1">{finding.location}</p>
            </div>
          </section>

          <section className="space-y-4">
            <div className="flex items-center gap-2">
              <Sparkles size={16} className="text-emerald-500" />
              <h3 className="font-bold text-sm">AI Analysis</h3>
            </div>
            
            {isLoading ? (
              <div className="flex flex-col items-center justify-center py-12 space-y-4 bg-zinc-50 dark:bg-zinc-950/50 rounded-2xl border border-dashed border-zinc-200 dark:border-zinc-800">
                <Loader2 className="w-8 h-8 text-emerald-500 animate-spin" />
                <p className="text-sm text-zinc-500 animate-pulse">Scanning vectors for code fixes...</p>
              </div>
            ) : (
              <div className="prose dark:prose-invert prose-sm max-w-none text-zinc-600 dark:text-zinc-400 leading-relaxed">
                {explanation ? (
                  <p>{explanation}</p>
                ) : (
                  <p className="text-red-500">Something went wrong. Could not analyze finding.</p>
                )}
              </div>
            )}
          </section>

          {fixCode && !isLoading && (
            <section className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full bg-emerald-500" />
                  <h3 className="font-bold text-sm">Proposed Code Fix</h3>
                </div>
                <button
                  onClick={copyCode}
                  className="flex items-center gap-2 text-xs font-bold text-zinc-500 hover:text-emerald-500 transition-colors"
                >
                  {copied ? <Check size={14} /> : <Copy size={14} />}
                  {copied ? "Copied!" : "Copy Fix"}
                </button>
              </div>
              <div className="rounded-xl overflow-hidden shadow-sm">
                <CodeSnippet code={fixCode} />
              </div>
              <p className="text-[10px] text-zinc-500 italic text-center">
                Always audit AI-generated code before production deployment.
              </p>
            </section>
          )}
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-zinc-200 dark:border-zinc-800 bg-zinc-50/50 dark:bg-zinc-950/50">
          <button
            onClick={onClose}
            className="w-full py-3 rounded-xl bg-zinc-900 dark:bg-white text-white dark:text-zinc-900 font-bold hover:scale-[1.02] transition-transform active:scale-95 shadow-xl"
          >
            I'll review this fix
          </button>
        </div>
      </div>
    </div>
  );
}
