"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  SHORTCUT_HELP,
  emitShortcut,
  isEditableTarget,
  resolveShortcut,
} from "../lib/keyboard-shortcuts";

/** Global shortcut listener plus the `?` help modal. Mounted once in the root layout. */
export function KeyboardShortcuts() {
  const router = useRouter();
  const [helpOpen, setHelpOpen] = useState(false);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (helpOpen && event.key === "Escape") {
        event.preventDefault();
        setHelpOpen(false);
        return;
      }

      const resolved = resolveShortcut({
        key: event.key,
        metaKey: event.metaKey,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
        isEditableTarget: isEditableTarget(event.target),
      });
      if (!resolved) return;

      event.preventDefault();
      if (resolved.kind === "help") setHelpOpen((open) => !open);
      else if (resolved.kind === "navigate") router.push(resolved.href);
      else emitShortcut(resolved.action);
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [helpOpen, router]);

  if (!helpOpen) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-start justify-center px-4 pt-[15vh]">
      <div
        className="absolute inset-0 bg-zinc-950/40 backdrop-blur-md"
        onClick={() => setHelpOpen(false)}
        aria-hidden="true"
      />
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Keyboard shortcuts"
        className="relative w-full max-w-md overflow-hidden rounded-2xl border border-white/20 bg-white/90 shadow-2xl backdrop-blur-xl dark:border-white/10 dark:bg-zinc-900/90 theme-high-contrast:border-white theme-high-contrast:bg-black"
      >
        <div className="flex items-center justify-between border-b border-zinc-200 px-4 py-3 dark:border-zinc-800">
          <h2 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">Keyboard shortcuts</h2>
          <button
            type="button"
            onClick={() => setHelpOpen(false)}
            className="rounded border border-zinc-300 px-1.5 py-0.5 text-[10px] text-zinc-500 dark:border-zinc-700"
          >
            esc
          </button>
        </div>
        <ul className="divide-y divide-zinc-100 p-2 dark:divide-zinc-800">
          {SHORTCUT_HELP.map((shortcut) => (
            <li key={shortcut.keys} className="flex items-center justify-between gap-4 px-2 py-2.5 text-sm">
              <span className="text-zinc-700 dark:text-zinc-300">{shortcut.description}</span>
              <kbd className="shrink-0 rounded border border-zinc-300 px-2 py-0.5 font-mono text-[11px] text-zinc-500 dark:border-zinc-700">
                {shortcut.keys}
              </kbd>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
