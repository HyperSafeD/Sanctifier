import Link from "next/link";

export default function NotFound() {
  return (
    <div className="flex min-h-[calc(100vh-64px)] flex-col items-center justify-center bg-zinc-50 dark:bg-zinc-950 px-6">
      <div className="max-w-md text-center">
        <h1 className="text-6xl font-bold text-zinc-300 dark:text-zinc-700 mb-4">
          404
        </h1>
        <h2 className="text-xl font-semibold text-zinc-900 dark:text-zinc-100 mb-2">
          Page not found
        </h2>
        <p className="text-zinc-600 dark:text-zinc-400 mb-6">
          The page you are looking for does not exist or has been moved.
        </p>
        <Link
          href="/"
          className="inline-block rounded-lg bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 px-6 py-3 font-medium hover:bg-zinc-800 dark:hover:bg-zinc-200 transition-colors"
        >
          Go back home
        </Link>
      </div>
    </div>
  );
}
