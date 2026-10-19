"use client";

import { useEffect } from "react";
import { useWorkspace } from "../providers/WorkspaceProvider";

interface WorkspaceSidebarProps {
  /** Mobile sheet open state (ignored on md+ where the sidebar is always visible). */
  isOpen?: boolean;
  /** Called when the user closes the mobile sheet (backdrop click or × button). */
  onClose?: () => void;
}

function SidebarContent({ onItemClick }: { onItemClick?: () => void }) {
  const { workspace, selectedContract, selectContract } = useWorkspace();

  if (!workspace || workspace.contracts.length <= 1) return null;

  return (
    <div className="space-y-4">
      <div className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4 shadow-sm">
        <h3 className="text-sm font-semibold mb-3 theme-high-contrast:text-yellow-300">
          Workspace Members
        </h3>
        <nav className="space-y-1 max-h-[calc(100vh-200px)] md:max-h-none overflow-y-auto">
          {workspace.contracts.map((contract) => (
            <button
              key={contract.name}
              onClick={() => {
                selectContract(contract.name);
                onItemClick?.();
              }}
              className={`w-full text-left px-3 py-2 rounded-lg text-sm transition-colors ${
                selectedContract?.name === contract.name
                  ? "bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 font-medium"
                  : "text-zinc-500 hover:bg-zinc-50 dark:hover:bg-zinc-800/50 hover:text-zinc-700 dark:hover:text-zinc-300"
              }`}
            >
              <div className="flex justify-between items-center">
                <span className="truncate">{contract.name}</span>
                {contract.total_findings > 0 && (
                  <span className="ml-2 px-1.5 py-0.5 rounded-full bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400 text-[10px] font-bold">
                    {contract.total_findings}
                  </span>
                )}
              </div>
            </button>
          ))}
        </nav>
      </div>

      {workspace.shared_libs.length > 0 && (
        <div className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4 shadow-sm opacity-60">
          <h3 className="text-sm font-semibold mb-2">Shared Libraries</h3>
          <ul className="space-y-1 max-h-[200px] overflow-y-auto">
            {workspace.shared_libs.map((lib) => (
              <li key={lib} className="px-3 py-1 text-xs text-zinc-500 truncate">
                📦 {lib}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

export function WorkspaceSidebar({ isOpen = false, onClose }: WorkspaceSidebarProps) {
  const { workspace } = useWorkspace();

  // Lock body scroll while the mobile sheet is open.
  useEffect(() => {
    document.body.style.overflow = isOpen ? "hidden" : "";
    return () => {
      document.body.style.overflow = "";
    };
  }, [isOpen]);

  if (!workspace || workspace.contracts.length <= 1) return null;

  return (
    <>
      {/* Desktop: permanent fixed-width sidebar */}
      <aside className="hidden md:block w-64 flex-shrink-0">
        <SidebarContent />
      </aside>

      {/* Mobile: slide-out sheet */}
      {isOpen && (
        <div className="md:hidden fixed inset-0 z-40 flex">
          {/* Backdrop */}
          <button
            aria-label="Close sidebar"
            className="absolute inset-0 bg-black/50"
            onClick={onClose}
          />

          {/* Sheet panel */}
          <div
            role="dialog"
            aria-label="Workspace members"
            className="relative z-50 w-64 max-w-[85vw] h-full bg-white dark:bg-zinc-900 border-r border-zinc-200 dark:border-zinc-800 overflow-y-auto p-4 flex flex-col gap-4 animate-in slide-in-from-left duration-300"
          >
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-semibold text-zinc-700 dark:text-zinc-300">
                Contracts
              </span>
              <button
                aria-label="Close sidebar"
                onClick={onClose}
                className="p-1 rounded hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors text-zinc-500"
              >
                ✕
              </button>
            </div>

            <SidebarContent onItemClick={onClose} />
          </div>
        </div>
      )}
    </>
  );
}
