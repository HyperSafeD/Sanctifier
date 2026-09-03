"use client";

import { useEffect, useCallback } from "react";
import { CodeSnippet } from "./CodeSnippet";
import { X } from "lucide-react";

interface SourceViewerProps {
  source: string;
  fileName?: string;
  highlightLine?: number;
  highlightEndLine?: number;
  onClose: () => void;
}

export function SourceViewer({
  source,
  fileName,
  highlightLine,
  highlightEndLine,
  onClose,
}: SourceViewerProps) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    },
    [onClose]
  );

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      document.body.style.overflow = "";
    };
  }, [handleKeyDown]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      role="dialog"
      aria-modal="true"
      aria-label={fileName ? `Source: ${fileName}` : "Source viewer"}
    >
      <div className="relative w-full max-w-5xl max-h-[85vh] mx-4 rounded-xl border border-zinc-700 bg-zinc-950 shadow-2xl flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-zinc-800 shrink-0">
          <div className="flex items-center gap-3 min-w-0">
            <span className="text-xs font-medium text-zinc-400 uppercase tracking-wider shrink-0">
              Source
            </span>
            {fileName && (
              <span className="text-sm text-zinc-300 truncate font-mono">
                {fileName}
              </span>
            )}
            {highlightLine && (
              <span className="text-xs text-amber-400 bg-amber-500/10 px-2 py-0.5 rounded shrink-0">
                Lines {highlightLine}
                {highlightEndLine ? `-${highlightEndLine}` : ""}
              </span>
            )}
          </div>
          <button
            onClick={onClose}
            aria-label="Close source viewer"
            className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 transition-colors shrink-0"
          >
            <X size={18} />
          </button>
        </div>

        {/* Code */}
        <div className="flex-1 overflow-hidden p-4">
          <CodeSnippet
            code={source}
            highlightLine={highlightLine}
            highlightEndLine={highlightEndLine}
            maxHeight="calc(85vh - 120px)"
          />
        </div>
      </div>
    </div>
  );
}
