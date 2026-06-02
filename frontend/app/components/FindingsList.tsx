"use client";

import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { FixedSizeList, type ListChildComponentProps } from "react-window";
import type { Finding, Severity } from "../types";
import { CodeSnippet } from "./CodeSnippet";
import { SourceViewer } from "./SourceViewer";
import { Sparkles, FileCode } from "lucide-react";
import { AiFixPanel } from "./AiFixPanel";
import { filterFindings } from "../lib/finding-filters";

interface FindingsListProps {
  findings: Finding[];
  severityFilter: Severity | "all";
  codeFilter?: string;
  getSource?: (finding: Finding) => string | undefined;
  getFileName?: (finding: Finding) => string | undefined;
}

/** Height reserved for each finding row in the virtual list (px). */
const ITEM_HEIGHT = 140;
/** Maximum height of the scrollable list before it starts scrolling. */
const MAX_LIST_HEIGHT = 600;
/** Below this threshold we skip virtualisation and render items directly. */
const VIRTUALISE_THRESHOLD = 50;

const severityColors: Record<Severity, string> = {
  critical: "bg-red-500/10 border-red-500/50 text-red-700 dark:text-red-400 theme-high-contrast:bg-black theme-high-contrast:border-white theme-high-contrast:text-white",
  high: "bg-orange-500/10 border-orange-500/50 text-orange-700 dark:text-orange-400 theme-high-contrast:bg-black theme-high-contrast:border-white theme-high-contrast:text-white",
  medium: "bg-amber-500/10 border-amber-500/50 text-amber-700 dark:text-amber-400 theme-high-contrast:bg-black theme-high-contrast:border-white theme-high-contrast:text-white",
  low: "bg-zinc-500/10 border-zinc-500/50 text-zinc-700 dark:text-zinc-400 theme-high-contrast:bg-black theme-high-contrast:border-yellow-300 theme-high-contrast:text-yellow-300",
};

const severityLabels: Record<Severity, string> = {
  critical: "Critical severity",
  high: "High severity",
  medium: "Medium severity",
  low: "Low severity",
};

interface FindingCardProps {
  finding: Finding;
  onSelectAiFix: (finding: Finding) => void;
  onViewSource?: (finding: Finding) => void;
}

function FindingCard({ finding, onSelectAiFix, onViewSource }: FindingCardProps) {
  return (
    <div className={`rounded-lg border p-4 ${severityColors[finding.severity]}`}>
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0 flex-1">
          <span className="text-xs font-semibold uppercase tracking-wide opacity-80">
            {finding.category}
          </span>
          <div className="flex items-center gap-3">
            <h3 className="mt-1 font-medium">{finding.title}</h3>
            <button 
              onClick={() => onSelectAiFix(finding)}
              aria-label="Get AI-powered fix suggestion for this finding"
              className="mt-1 flex items-center gap-1.5 px-2 py-1 rounded-md bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 text-[10px] font-bold border border-emerald-500/20 hover:bg-emerald-500/20 transition-colors"
            >
              <Sparkles size={10} />
              ASK AI
            </button>
          </div>
          <p className="mt-1 text-sm opacity-90">{finding.location}</p>
          {finding.suggestion && (
            <p className="mt-2 text-sm italic">💡 {finding.suggestion}</p>
          )}
        </div>
        <div className="shrink-0 flex items-center gap-2">
          {onViewSource && (
            <button
              onClick={() => onViewSource(finding)}
              aria-label="View full source file"
              className="flex items-center gap-1 px-2 py-1 rounded-md bg-zinc-500/10 text-zinc-400 dark:text-zinc-400 text-[10px] font-bold border border-zinc-500/20 hover:bg-zinc-500/20 transition-colors"
              title="View full source file"
            >
              <FileCode size={10} />
              SOURCE
            </button>
          )}
          <span
            className={`rounded px-2 py-1 text-xs font-medium border ${severityColors[finding.severity]}`}
            aria-label={severityLabels[finding.severity]}
          >
            {finding.severity}
          </span>
          <span className="font-mono text-xs rounded border border-zinc-300/70 dark:border-zinc-600 px-2 py-1 text-zinc-700 dark:text-zinc-300 theme-high-contrast:border-white theme-high-contrast:text-white" aria-label={`Error code: ${finding.code}`}>
            {finding.code}
          </span>
        </div>
      </div>
      {finding.snippet && (
        <div className="mt-3">
          <CodeSnippet code={finding.snippet} highlightLine={finding.line} />
        </div>
      )}
    </div>
  );
}

export function FindingsList({ findings, severityFilter, codeFilter = "", getSource, getFileName }: FindingsListProps) {
  const [selectedFinding, setSelectedFinding] = useState<Finding | null>(null);
  const [sourceViewerFinding, setSourceViewerFinding] = useState<Finding | null>(null);
  const listRef = useRef<FixedSizeList>(null);
  
  const filtered = useMemo(() => {
    return filterFindings(findings, severityFilter, codeFilter);
  }, [codeFilter, findings, severityFilter]);

  // Scroll back to top whenever the filter changes.
  useEffect(() => {
    listRef.current?.scrollToItem(0);
  }, [severityFilter, codeFilter]);

  const handleViewSource = useCallback((finding: Finding) => {
    setSourceViewerFinding(finding);
  }, []);

  const handleCloseSource = useCallback(() => {
    setSourceViewerFinding(null);
  }, []);

  const Row = useCallback(
    ({ index, style }: ListChildComponentProps) => (
      <div style={{ ...style, paddingBottom: 16 }}>
        <FindingCard 
          finding={filtered[index]} 
          onSelectAiFix={(f) => setSelectedFinding(f)} 
          onViewSource={getSource ? handleViewSource : undefined}
        />
      </div>
    ),
    [filtered, getSource, handleViewSource],
  );

  if (filtered.length === 0) {
    return (
      <p className="text-zinc-500 dark:text-zinc-400 theme-high-contrast:text-white py-8 text-center">
        No findings match the selected filter.
      </p>
    );
  }

  const sourceViewerSource = sourceViewerFinding && getSource ? getSource(sourceViewerFinding) : undefined;

  return (
    <div className="relative">
      {sourceViewerSource && sourceViewerFinding && (
        <SourceViewer
          source={sourceViewerSource}
          fileName={getFileName?.(sourceViewerFinding) ?? "contract.rs"}
          highlightLine={sourceViewerFinding.line}
          highlightEndLine={sourceViewerFinding.line}
          onClose={handleCloseSource}
        />
      )}
      <AiFixPanel finding={selectedFinding} onClose={() => setSelectedFinding(null)} />

      {/* For small lists render items directly — no virtualisation overhead. */}
      {filtered.length < VIRTUALISE_THRESHOLD ? (
        <div className="space-y-4">
          {filtered.map((f) => (
            <FindingCard 
              key={f.id} 
              finding={f} 
              onSelectAiFix={(finding) => setSelectedFinding(finding)} 
              onViewSource={getSource ? handleViewSource : undefined}
            />
          ))}
        </div>
      ) : (
        <>
          {/* For large lists (1000+) use a fixed-size virtual window. */}
          <FixedSizeList
            height={Math.min(filtered.length * ITEM_HEIGHT, MAX_LIST_HEIGHT)}
            itemCount={filtered.length}
            itemSize={ITEM_HEIGHT}
            width="100%"
            ref={listRef}
          >
            {Row}
          </FixedSizeList>
        </>
      )}
    </div>
  );
}
