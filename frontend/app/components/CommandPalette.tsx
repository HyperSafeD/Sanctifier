"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { Search } from "lucide-react";
import { useTheme } from "../providers/theme-provider";
import {
  FINDING_CODE_ITEMS,
  PAGE_ITEMS,
  SETTING_ITEMS,
  contractItem,
  pushRecentId,
  readRecentIds,
  searchItems,
  type PaletteItem,
} from "../lib/command-palette";
import type { ContractInfo } from "../lib/contracts-model";

const LISTBOX_ID = "command-palette-listbox";

export function CommandPalette() {
  const router = useRouter();
  const { toggleTheme } = useTheme();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [contracts, setContracts] = useState<PaletteItem[]>([]);
  const [recentIds, setRecentIds] = useState<string[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);
  const contractsRequested = useRef(false);

  const openPalette = useCallback(() => {
    setQuery("");
    setActiveIndex(0);
    setRecentIds(readRecentIds());
    setOpen(true);

    if (contractsRequested.current) return;
    contractsRequested.current = true;
    fetch("/api/contracts")
      .then((res) => (res.ok ? res.json() : null))
      .then((data: { contracts?: ContractInfo[] } | null) => {
        if (data?.contracts) {
          setContracts(data.contracts.map((c) => contractItem(c.name, c.description)));
        }
      })
      .catch(() => {
        contractsRequested.current = false;
      });
  }, []);

  const closePalette = useCallback(() => setOpen(false), []);

  // Global ⌘K / Ctrl+K shortcut
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        if (open) closePalette();
        else openPalette();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, openPalette, closePalette]);

  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  const allItems = useMemo(
    () => [...PAGE_ITEMS, ...SETTING_ITEMS, ...contracts, ...FINDING_CODE_ITEMS],
    [contracts],
  );

  const { results, heading } = useMemo(() => {
    if (query.trim()) return { results: searchItems(allItems, query), heading: "Results" };
    const recent = recentIds
      .map((id) => allItems.find((item) => item.id === id))
      .filter((item): item is PaletteItem => Boolean(item));
    return recent.length
      ? { results: recent, heading: "Recent" }
      : { results: PAGE_ITEMS, heading: "Suggested" };
  }, [allItems, query, recentIds]);

  const active = results.length ? Math.min(activeIndex, results.length - 1) : -1;

  useEffect(() => {
    if (active < 0) return;
    document.getElementById(`palette-option-${active}`)?.scrollIntoView?.({ block: "nearest" });
  }, [active]);

  const select = (item: PaletteItem) => {
    setRecentIds(pushRecentId(item.id));
    closePalette();
    if (item.action === "toggle-theme") {
      toggleTheme();
    } else if (item.href?.startsWith("http")) {
      window.open(item.href, "_blank", "noopener,noreferrer");
    } else if (item.href) {
      router.push(item.href);
    }
  };

  const onInputKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (results.length) setActiveIndex((active + 1) % results.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      if (results.length) setActiveIndex((active - 1 + results.length) % results.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (active >= 0) select(results[active]);
    } else if (event.key === "Escape") {
      event.preventDefault();
      closePalette();
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-start justify-center px-4 pt-[15vh]">
      <div
        className="absolute inset-0 bg-zinc-950/40 backdrop-blur-md"
        onClick={closePalette}
        aria-hidden="true"
      />
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
        className="palette-panel relative w-full max-w-xl overflow-hidden rounded-2xl border border-white/20 dark:border-white/10 bg-white/80 dark:bg-zinc-900/80 theme-high-contrast:bg-black theme-high-contrast:border-white shadow-2xl backdrop-blur-xl"
      >
        <div className="flex items-center gap-3 border-b border-zinc-200 dark:border-zinc-800 px-4">
          <Search className="h-4 w-4 shrink-0 text-zinc-400" aria-hidden="true" />
          <input
            ref={inputRef}
            type="text"
            role="combobox"
            aria-expanded="true"
            aria-controls={LISTBOX_ID}
            aria-activedescendant={active >= 0 ? `palette-option-${active}` : undefined}
            aria-label="Search pages, contracts, finding codes, and settings"
            placeholder="Search pages, contracts, finding codes, settings…"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setActiveIndex(0);
            }}
            onKeyDown={onInputKeyDown}
            className="h-12 w-full bg-transparent text-sm text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 focus:outline-none"
          />
          <kbd className="hidden sm:inline rounded border border-zinc-300 dark:border-zinc-700 px-1.5 py-0.5 text-[10px] text-zinc-500">
            esc
          </kbd>
        </div>

        <div className="max-h-80 overflow-y-auto p-2">
          <p className="px-2 pb-1 pt-1 text-xs font-medium uppercase tracking-wide text-zinc-400">{heading}</p>
          {results.length === 0 ? (
            <p className="px-3 py-8 text-center text-sm text-zinc-500">No results for “{query}”</p>
          ) : (
            <ul id={LISTBOX_ID} role="listbox" aria-label={heading}>
              {results.map((item, index) => (
                <li
                  key={item.id}
                  id={`palette-option-${index}`}
                  role="option"
                  aria-selected={index === active}
                  onMouseMove={() => setActiveIndex(index)}
                  onClick={() => select(item)}
                  className={`flex cursor-pointer items-center justify-between gap-3 rounded-lg px-3 py-2 text-sm ${
                    index === active
                      ? "bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-900 theme-high-contrast:bg-yellow-300 theme-high-contrast:text-black"
                      : "text-zinc-700 dark:text-zinc-300"
                  }`}
                >
                  <span className="min-w-0">
                    <span className="block truncate font-medium">{item.label}</span>
                    {item.description && (
                      <span className="block truncate text-xs opacity-70">{item.description}</span>
                    )}
                  </span>
                  <span className="shrink-0 rounded-full border border-current/20 px-2 py-0.5 text-[10px] uppercase tracking-wide opacity-70">
                    {item.category}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="flex items-center gap-4 border-t border-zinc-200 dark:border-zinc-800 px-4 py-2 text-[11px] text-zinc-500">
          <span>↑↓ navigate</span>
          <span>↵ select</span>
          <span>esc close</span>
        </div>
      </div>
    </div>
  );
}
