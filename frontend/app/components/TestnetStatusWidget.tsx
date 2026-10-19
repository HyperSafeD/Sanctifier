"use client";

import { useEffect, useState, useCallback } from "react";
import type { TestnetStatusResponse, ContractStatus } from "../api/testnet-status/route";

const REFRESH_INTERVAL_MS = 30_000;

type WidgetState =
  | { phase: "loading" }
  | { phase: "error"; message: string }
  | { phase: "ok"; data: TestnetStatusResponse };

function StatusDot({ alive }: { alive: boolean }) {
  return (
    <span
      aria-label={alive ? "Online" : "Offline"}
      className={[
        "inline-block w-2.5 h-2.5 rounded-full flex-shrink-0",
        alive
          ? "bg-emerald-500 shadow-[0_0_6px_2px_rgba(16,185,129,0.5)]"
          : "bg-red-500",
      ].join(" ")}
    />
  );
}

function ContractRow({ contract }: { contract: ContractStatus }) {
  const short = `${contract.address.slice(0, 6)}…${contract.address.slice(-4)}`;
  return (
    <div className="flex items-start gap-3 py-3 border-b last:border-0 border-zinc-200 dark:border-zinc-800">
      <StatusDot alive={contract.alive} />
      <div className="flex-1 min-w-0">
        <p className="font-medium text-sm text-zinc-900 dark:text-zinc-100">
          {contract.label}
        </p>
        <a
          href={contract.explorerUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="text-xs text-zinc-500 dark:text-zinc-400 hover:text-emerald-600 dark:hover:text-emerald-400 font-mono truncate block"
          aria-label={`View ${contract.label} on Stellar Expert`}
        >
          {short}
        </a>
        {contract.errorMessage && (
          <p className="text-xs text-red-500 mt-0.5">{contract.errorMessage}</p>
        )}
      </div>
      <span
        className={[
          "text-xs font-semibold px-2 py-0.5 rounded-full flex-shrink-0",
          contract.alive
            ? "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-400"
            : "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-400",
        ].join(" ")}
      >
        {contract.alive ? "Live" : "Offline"}
      </span>
    </div>
  );
}

export function TestnetStatusWidget() {
  const [state, setState] = useState<WidgetState>({ phase: "loading" });
  const [lastRefreshed, setLastRefreshed] = useState<Date | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const res = await fetch("/api/testnet-status");
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      const data = (await res.json()) as TestnetStatusResponse;
      setState({ phase: "ok", data });
      setLastRefreshed(new Date());
    } catch (err) {
      setState({
        phase: "error",
        message: err instanceof Error ? err.message : "Failed to load status",
      });
    }
  }, []);

  useEffect(() => {
    void fetchStatus();
    const id = setInterval(() => void fetchStatus(), REFRESH_INTERVAL_MS);
    return () => clearInterval(id);
  }, [fetchStatus]);

  return (
    <section
      aria-label="Testnet contract status"
      className="rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white/60 dark:bg-zinc-900/60 backdrop-blur-sm p-5 w-full"
    >
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100 flex items-center gap-2">
          {state.phase === "ok" && (
            <StatusDot alive={state.data.networkHealthy} />
          )}
          Soroban Testnet Status
        </h2>
        {state.phase === "ok" && state.data.ledger !== undefined && (
          <span className="text-xs text-zinc-500 dark:text-zinc-400 tabular-nums">
            Ledger #{state.data.ledger.toLocaleString()}
          </span>
        )}
      </div>

      {/* Body */}
      {state.phase === "loading" && (
        <div
          role="status"
          aria-label="Loading testnet status"
          className="flex items-center gap-2 py-4 text-sm text-zinc-500 dark:text-zinc-400"
        >
          <svg
            className="animate-spin w-4 h-4 text-emerald-500"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            />
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
            />
          </svg>
          Checking contract status…
        </div>
      )}

      {state.phase === "error" && (
        <p
          role="alert"
          className="text-sm text-red-600 dark:text-red-400 py-2"
        >
          Unable to fetch status: {state.message}
        </p>
      )}

      {state.phase === "ok" && (
        <div>
          {state.data.contracts.map((c) => (
            <ContractRow key={c.id} contract={c} />
          ))}
        </div>
      )}

      {/* Footer */}
      {lastRefreshed && (
        <p className="mt-3 text-xs text-zinc-400 dark:text-zinc-500">
          Last updated:{" "}
          <time dateTime={lastRefreshed.toISOString()}>
            {lastRefreshed.toLocaleTimeString()}
          </time>
          {" · "}
          <button
            type="button"
            onClick={() => void fetchStatus()}
            className="underline hover:text-zinc-600 dark:hover:text-zinc-300 cursor-pointer"
          >
            Refresh
          </button>
        </p>
      )}
    </section>
  );
}
