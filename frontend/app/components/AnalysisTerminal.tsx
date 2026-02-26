"use client";

import React, { useEffect, useRef, useState } from "react";

interface LogEntry {
  id: string;
  text: string;
  timestamp: Date;
  type: "info" | "error" | "warning" | "success";
}

interface AnalysisTerminalProps {
  logs: string[];
  isAnalyzing: boolean;
}

const parseAnsi = (text: string) => {
  // Simple ANSI stripper for now, can be expanded to support actual colors if needed
  // eslint-disable-next-line no-control-regex
  return text.replace(/\x1b\[[0-9;]*[mK]/g, "");
};

export const AnalysisTerminal: React.FC<AnalysisTerminalProps> = ({ logs, isAnalyzing }) => {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  useEffect(() => {
    if (autoScroll && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  const handleScroll = () => {
    if (!scrollRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
    const isAtBottom = scrollHeight - scrollTop - clientHeight < 10;
    setAutoScroll(isAtBottom);
  };

  return (
    <div className="flex flex-col h-[500px] w-full rounded-xl border border-zinc-200 dark:border-zinc-800 bg-zinc-950 text-zinc-300 font-mono text-sm overflow-hidden shadow-2xl relative group">
      {/* Terminal Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-zinc-900 bg-zinc-900/50 backdrop-blur-md">
        <div className="flex gap-1.5">
          <div className="w-3 h-3 rounded-full bg-red-500/80" />
          <div className="w-3 h-3 rounded-full bg-amber-500/80" />
          <div className="w-3 h-3 rounded-full bg-emerald-500/80" />
        </div>
        <div className="flex items-center gap-2 text-xs text-zinc-500">
          <span className="animate-pulse flex items-center gap-1">
            {isAnalyzing && (
              <>
                <div className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                ANALYZING...
              </>
            )}
            {!isAnalyzing && "IDLE"}
          </span>
          <span className="opacity-50">|</span>
          <span>bash — 80x24</span>
        </div>
      </div>

      {/* Terminal Content */}
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto p-4 space-y-1 scrollbar-thin scrollbar-thumb-zinc-800 scrollbar-track-transparent custom-scrollbar"
      >
        {logs.length === 0 ? (
          <div className="text-zinc-600 italic">No output yet. Start an analysis to see logs...</div>
        ) : (
          logs.map((log, i) => (
            <div key={i} className="flex gap-4 group/line">
              <span className="text-zinc-700 select-none w-8 text-right shrink-0">{i + 1}</span>
              <span className="whitespace-pre-wrap break-all inline-block truncate-line">
                {parseAnsi(log)}
              </span>
            </div>
          ))
        )}
        {isAnalyzing && (
          <div className="flex gap-4 animate-in fade-in slide-in-from-left-2 transition-all">
            <span className="text-zinc-700 select-none w-8 text-right shrink-0">{logs.length + 1}</span>
            <span className="w-2 h-5 bg-emerald-500/80 animate-pulse inline-block" />
          </div>
        )}
      </div>

      {/* Floating Controls */}
      <div className="absolute bottom-4 right-4 opacity-0 group-hover:opacity-100 transition-opacity">
        <button
          onClick={() => {
            if (scrollRef.current) {
              scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
              setAutoScroll(true);
            }
          }}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium transition-colors ${
            autoScroll
              ? "bg-emerald-500/10 text-emerald-500"
              : "bg-white/10 text-zinc-400 hover:bg-white/20"
          } backdrop-blur-md border border-white/5`}
        >
          {autoScroll ? "Live Tracking ON" : "Jump to Bottom"}
        </button>
      </div>

      <style jsx>{`
        .custom-scrollbar::-webkit-scrollbar {
          width: 8px;
        }
        .custom-scrollbar::-webkit-scrollbar-track {
          background: transparent;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb {
          background: rgba(39, 39, 42, 0.5);
          border-radius: 10px;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb:hover {
          background: rgba(63, 63, 70, 0.5);
        }
        .truncate-line {
           /* Could add more styling here if needed */
        }
      `}</style>
    </div>
  );
};
