"use client";

import { useEffect } from "react";

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error("Route error:", error);
  }, [error]);

  const handleReload = () => {
    window.location.reload();
  };

  const handleCopyDetails = () => {
    const details = `Error: ${error.message}\n\nDigest: ${error.digest ?? "N/A"}\n\nStack:\n${error.stack}`;
    navigator.clipboard.writeText(details).then(
      () => alert("Error details copied to clipboard"),
      () => alert("Failed to copy error details")
    );
  };

  return (
    <div
      role="alert"
      className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900 p-6"
    >
      <div className="max-w-md w-full rounded-lg border border-red-300 dark:border-red-700 bg-white dark:bg-gray-800 p-8 text-center shadow-lg">
        <div className="mb-4">
          <svg
            className="mx-auto h-12 w-12 text-red-500"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
        </div>
        <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100 mb-2">
          Something went wrong
        </h2>
        <p className="text-sm text-gray-600 dark:text-gray-400 mb-6">
          {error.message || "An unexpected error occurred. Please try reloading the page."}
        </p>
        <div className="flex flex-col gap-3">
          <button
            onClick={handleReload}
            className="w-full rounded-lg bg-red-600 text-white px-4 py-2.5 text-sm font-medium hover:bg-red-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-500 focus-visible:ring-offset-2 transition-colors"
          >
            Reload Page
          </button>
          <button
            onClick={reset}
            className="w-full rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 px-4 py-2.5 text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-gray-500 focus-visible:ring-offset-2 transition-colors"
          >
            Try Again
          </button>
          <button
            onClick={handleCopyDetails}
            className="w-full rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 px-4 py-2.5 text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-gray-500 focus-visible:ring-offset-2 transition-colors"
          >
            Copy Error Details
          </button>
        </div>
      </div>
    </div>
  );
}
