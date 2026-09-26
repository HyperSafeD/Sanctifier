"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { Search } from "lucide-react";
import { RiskBadge } from "../components/RiskBadge";
import {
  DEFAULT_CONTRACT_FILTERS,
  RISK_ORDER,
  filterAndSortContracts,
  type ContractInfo,
  type ContractSortKey,
  type RiskLevel,
} from "../lib/contracts-model";

const CONTROL_CLASS =
  "rounded-lg border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 text-sm text-zinc-900 dark:text-zinc-100 focus:outline-none focus:ring-2 focus:ring-zinc-500";

export function ContractsExplorer({ contracts }: { contracts: ContractInfo[] }) {
  const [query, setQuery] = useState(DEFAULT_CONTRACT_FILTERS.query);
  const [risk, setRisk] = useState<RiskLevel | "all">(DEFAULT_CONTRACT_FILTERS.risk);
  const [sort, setSort] = useState<ContractSortKey>(DEFAULT_CONTRACT_FILTERS.sort);

  const visible = useMemo(
    () => filterAndSortContracts(contracts, { query, risk, sort }),
    [contracts, query, risk, sort],
  );

  return (
    <div>
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
        <label className="relative flex-1">
          <span className="sr-only">Search contracts</span>
          <Search
            className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-zinc-400"
            aria-hidden="true"
          />
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search by name, tag, or description…"
            className={`${CONTROL_CLASS} w-full pl-9`}
          />
        </label>
        <label className="flex items-center gap-2 text-sm text-zinc-600 dark:text-zinc-400">
          Risk
          <select
            value={risk}
            onChange={(e) => setRisk(e.target.value as RiskLevel | "all")}
            className={CONTROL_CLASS}
          >
            <option value="all">All levels</option>
            {RISK_ORDER.map((level) => (
              <option key={level} value={level} className="capitalize">
                {level}
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center gap-2 text-sm text-zinc-600 dark:text-zinc-400">
          Sort by
          <select
            value={sort}
            onChange={(e) => setSort(e.target.value as ContractSortKey)}
            className={CONTROL_CLASS}
          >
            <option value="risk">Risk</option>
            <option value="findings">Findings</option>
            <option value="loc">Lines of code</option>
          </select>
        </label>
      </div>

      <p className="mt-4 text-sm text-zinc-500 dark:text-zinc-400" aria-live="polite">
        Showing {visible.length} of {contracts.length} contracts
      </p>

      {visible.length === 0 ? (
        <div className="mt-6 rounded-xl border border-dashed border-zinc-300 dark:border-zinc-700 p-10 text-center text-zinc-500">
          No contracts match your filters.
        </div>
      ) : (
        <ul className="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
          {visible.map((contract) => (
            <li
              key={contract.name}
              className="flex flex-col rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-5 shadow-sm theme-high-contrast:border-white"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <h2 className="truncate text-base font-semibold text-zinc-900 dark:text-zinc-50">
                    <Link href={`/contracts/${encodeURIComponent(contract.name)}`} className="hover:underline">
                      {contract.name}
                    </Link>
                  </h2>
                  <p className="truncate font-mono text-xs text-zinc-500">{contract.path}</p>
                </div>
                <RiskBadge risk={contract.risk} />
              </div>

              <p className="mt-3 line-clamp-2 text-sm text-zinc-600 dark:text-zinc-400">{contract.description}</p>

              <div className="mt-3 flex flex-wrap gap-1.5">
                {contract.tags.map((tag) => (
                  <span
                    key={tag}
                    className="rounded-md bg-zinc-100 dark:bg-zinc-800 px-2 py-0.5 text-xs text-zinc-600 dark:text-zinc-300"
                  >
                    {tag}
                  </span>
                ))}
              </div>

              <div className="mt-auto flex items-center justify-between pt-4 text-xs text-zinc-500">
                <span>
                  <strong className="text-zinc-900 dark:text-zinc-100">{contract.findingCount}</strong>{" "}
                  {contract.findingCount === 1 ? "finding" : "findings"} · {contract.linesOfCode} LOC
                </span>
                <Link
                  href={`/scan?contract=${encodeURIComponent(contract.name)}`}
                  className="rounded-md bg-zinc-900 px-3 py-1.5 font-medium text-white hover:bg-zinc-700 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-300"
                >
                  Scan
                </Link>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
