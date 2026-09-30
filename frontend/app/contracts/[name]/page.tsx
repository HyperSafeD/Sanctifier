import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Breadcrumbs } from "../../components/Breadcrumbs";
import { RiskBadge } from "../../components/RiskBadge";
import { loadContractByName } from "../../lib/contracts-catalog";

interface PageProps {
  params: Promise<{ name: string }>;
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { name } = await params;
  return { title: `${name} | Contracts | Sanctifier` };
}

export default async function ContractDetailPage({ params }: PageProps) {
  const { name } = await params;
  const contract = await loadContractByName(name);
  if (!contract) notFound();

  return (
    <>
      <Breadcrumbs />
      <main className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h1 className="text-2xl font-bold text-zinc-900 dark:text-zinc-50">{contract.name}</h1>
            <p className="mt-1 font-mono text-xs text-zinc-500">{contract.path}</p>
          </div>
          <RiskBadge risk={contract.risk} />
        </div>

        <p className="mt-4 text-zinc-700 dark:text-zinc-300">{contract.description}</p>

        <dl className="mt-6 grid grid-cols-2 gap-4 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-5">
          <div>
            <dt className="text-xs uppercase tracking-wide text-zinc-500">Pre-scan findings</dt>
            <dd className="text-xl font-semibold text-zinc-900 dark:text-zinc-50">{contract.findingCount}</dd>
          </div>
          <div>
            <dt className="text-xs uppercase tracking-wide text-zinc-500">Lines of code</dt>
            <dd className="text-xl font-semibold text-zinc-900 dark:text-zinc-50">{contract.linesOfCode}</dd>
          </div>
        </dl>

        {contract.tags.length > 0 && (
          <div className="mt-4 flex flex-wrap gap-1.5">
            {contract.tags.map((tag) => (
              <span
                key={tag}
                className="rounded-md bg-zinc-100 dark:bg-zinc-800 px-2 py-0.5 text-xs text-zinc-600 dark:text-zinc-300"
              >
                {tag}
              </span>
            ))}
          </div>
        )}

        <div className="mt-8 flex gap-3">
          <Link
            href={`/scan?contract=${encodeURIComponent(contract.name)}`}
            className="rounded-lg bg-zinc-900 px-4 py-2 text-sm font-medium text-white hover:bg-zinc-700 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-300"
          >
            Scan this contract
          </Link>
          <Link
            href="/contracts"
            className="rounded-lg border border-zinc-300 dark:border-zinc-700 px-4 py-2 text-sm font-medium text-zinc-700 dark:text-zinc-300 hover:bg-zinc-50 dark:hover:bg-zinc-800"
          >
            Back to contracts
          </Link>
        </div>
      </main>
    </>
  );
}
