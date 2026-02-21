import Link from "next/link";

export default function Home() {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-zinc-50 dark:bg-zinc-950 font-sans">
      <main className="flex flex-col items-center gap-8 px-6">
        <h1 className="text-4xl font-bold text-zinc-900 dark:text-zinc-50">
          Sanctifier
        </h1>
        <p className="text-lg text-zinc-600 dark:text-zinc-400 text-center max-w-md">
          Stellar Soroban Security & Formal Verification Suite
        </p>
        <Link
          href="/dashboard"
          className="rounded-lg bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 px-6 py-3 font-medium hover:bg-zinc-800 dark:hover:bg-zinc-200 transition-colors"
        >
          Open Security Dashboard
        </Link>
      </main>
    </div>
  );
}
