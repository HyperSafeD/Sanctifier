import type { Metadata } from "next";
import { Breadcrumbs } from "../components/Breadcrumbs";
import { loadContracts } from "../lib/contracts-catalog";
import { ContractsExplorer } from "./ContractsExplorer";

export const metadata: Metadata = {
  title: "Contracts Explorer | Sanctifier",
  description: "Browse the workspace's Soroban contracts with risk badges, search, and filters.",
};

export default async function ContractsPage() {
  const contracts = await loadContracts();

  return (
    <>
      <Breadcrumbs />
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <header className="mb-6">
          <h1 className="text-2xl font-bold text-zinc-900 dark:text-zinc-50">Contracts Explorer</h1>
          <p className="mt-1 text-sm text-zinc-600 dark:text-zinc-400">
            Risk badges come from a quick source pre-scan (panics, unsafe blocks, raw cross-contract calls).
            Open a contract in Scan for a full analysis.
          </p>
        </header>

        {contracts.length === 0 ? (
          <div className="rounded-xl border border-dashed border-zinc-300 dark:border-zinc-700 p-10 text-center text-zinc-500">
            No contracts were found in the workspace <code>contracts/</code> directory.
          </div>
        ) : (
          <ContractsExplorer contracts={contracts} />
        )}
      </main>
    </>
  );
}
