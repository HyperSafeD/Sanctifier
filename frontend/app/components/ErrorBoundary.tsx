"use client";

import { Component } from "react";
import type { ReactNode, ErrorInfo } from "react";

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
  /**
   * Render a small inline fallback instead of the default full-screen
   * overlay (issue #1448). The default `min-h-screen` fallback assumes this
   * boundary wraps the entire page; several call sites (e.g.
   * `app/scan/page.tsx`) instead wrap a single section of a larger page, so
   * a render error there shouldn't cover the whole viewport with an
   * unrelated error card. Also offers "Try Again" (re-render this subtree
   * only) alongside "Reload Page", instead of only the latter.
   */
  compact?: boolean;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("ErrorBoundary caught an error:", error, errorInfo);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  handleReload = () => {
    window.location.reload();
  };

  handleCopyDetails = () => {
    const details = `Error: ${this.state.error?.message}\n\nStack:\n${this.state.error?.stack}`;
    navigator.clipboard.writeText(details).then(
      () => alert("Error details copied to clipboard"),
      () => alert("Failed to copy error details")
    );
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      if (this.props.compact) {
        return (
          <div
            role="alert"
            className="rounded-lg border border-red-300 dark:border-red-700 bg-white dark:bg-gray-800 p-6 text-center"
          >
            <p className="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-1">
              This section couldn&apos;t be displayed
            </p>
            <p className="text-xs text-gray-600 dark:text-gray-400 mb-4">
              {this.state.error?.message ?? "An unexpected error occurred."}
            </p>
            <div className="flex flex-wrap items-center justify-center gap-2">
              <button
                onClick={this.handleReset}
                className="rounded-lg bg-red-600 text-white px-3 py-1.5 text-xs font-medium hover:bg-red-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-500 focus-visible:ring-offset-2 transition-colors"
              >
                Try Again
              </button>
              <button
                onClick={this.handleCopyDetails}
                className="rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 px-3 py-1.5 text-xs font-medium hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-gray-500 focus-visible:ring-offset-2 transition-colors"
              >
                Copy Error Details
              </button>
            </div>
          </div>
        );
      }

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
              {this.state.error?.message ?? "An unexpected error occurred. Please try reloading the page."}
            </p>
            <div className="flex flex-col gap-3">
              <button
                onClick={this.handleReset}
                className="w-full rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 px-4 py-2.5 text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-gray-500 focus-visible:ring-offset-2 transition-colors"
              >
                Try Again
              </button>
              <button
                onClick={this.handleReload}
                className="w-full rounded-lg bg-red-600 text-white px-4 py-2.5 text-sm font-medium hover:bg-red-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-500 focus-visible:ring-offset-2 transition-colors"
              >
                Reload Page
              </button>
              <button
                onClick={this.handleCopyDetails}
                className="w-full rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 px-4 py-2.5 text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-gray-500 focus-visible:ring-offset-2 transition-colors"
              >
                Copy Error Details
              </button>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
