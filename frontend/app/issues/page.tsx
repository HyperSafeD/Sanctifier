"use client";

import { useEffect, useMemo, useState } from "react";
import { ExternalLink } from "lucide-react";
import { Breadcrumbs } from "../components/Breadcrumbs";
import { Skeleton } from "../components/Skeleton";
import {
  ISSUE_CATEGORIES,
  PAGE_SIZE,
  SEVERITIES,
  filterIssues,
  pageCount,
  paginate,
  sortIssues,
  type IssueCategory,
  type IssueRow,
  type IssueSeverity,
  type IssueSort,
} from "../lib/github-issues";

const SEVERITY_TEXT: Record<IssueSeverity, string> = {
  critical: "text-red-700 dark:text-red-400",
  high: "text-orange-700 dark:text-orange-400",
  medium: "text-amber-700 dark:text-amber-400",
  low: "text-emerald-700 dark:text-emerald-400",
  none: "text-zinc-500",
};

const SORT_OPTIONS: Array<{ value: IssueSort; label: string }> = [
  { value: "created-desc", label: "Newest first" },
  { value: "created-asc", label: "Oldest first" },
  { value: "severity", label: "Severity" },
];

const SELECT_CLASS =
  "rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-700 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-300";

function formatDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? "—" : date.toISOString().slice(0, 10);
}

export default function IssuesPage() {
  const [issues, setIssues] = useState<IssueRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [repo, setRepo] = useState("");
  const [category, setCategory] = useState<IssueCategory | "all">("all");
  const [severity, setSeverity] = useState<IssueSeverity | "all">("all");
  const [sort, setSort] = useState<IssueSort>("created-desc");
  const [page, setPage] = useState(1);

  useEffect(() => {
    let cancelled = false;
    fetch("/api/issues")
      .then(async (res) => {
        const data = (await res.json()) as { issues?: IssueRow[]; repo?: string; message?: string };
        if (!res.ok) throw new Error(data.message ?? "Could not load issues");
        return data;
      })
      .then((data) => {
        if (cancelled) return;
        setIssues(data.issues ?? []);
        setRepo(data.repo ?? "");
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setIssues([]);
        setError(err instanceof Error ? err.message : "Could not load issues");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const visible = useMemo(
    () => sortIssues(filterIssues(issues ?? [], { category, severity }), sort),
    [issues, category, severity, sort],
  );

  const totalPages = pageCount(visible.length);
  const currentPage = Math.min(page, totalPages);
  const rows = paginate(visible, currentPage);

  return (
    <>
      <Breadcrumbs />
      <main className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-6">
        <header>
          <h1 className="text-2xl font-bold text-zinc-900 dark:text-zinc-50">Issue Tracker</h1>
          <p className="mt-1 text-sm text-zinc-600 dark:text-zinc-400">
            Open and closed issues from {repo || "the Sanctifier repository"}. Select a row to open it on GitHub.
          </p>
        </header>

        <div className="flex flex-wrap items-center gap-3">
          <label className="flex items-center gap-2 text-sm text-zinc-600 dark:text-zinc-400">
            Label
            <select
              value={category}
              onChange={(e) => {
                setCategory(e.target.value as IssueCategory | "all");
                setPage(1);
              }}
              className={SELECT_CLASS}
            >
              <option value="all">All labels</option>
              {ISSUE_CATEGORIES.map((value) => (
                <option key={value} value={value} className="capitalize">
                  {value}
                </option>
              ))}
            </select>
          </label>

          <label className="flex items-center gap-2 text-sm text-zinc-600 dark:text-zinc-400">
            Severity
            <select
              value={severity}
              onChange={(e) => {
                setSeverity(e.target.value as IssueSeverity | "all");
                setPage(1);
              }}
              className={SELECT_CLASS}
            >
              <option value="all">All severities</option>
              {SEVERITIES.map((value) => (
                <option key={value} value={value}>
                  {value === "none" ? "Unlabelled" : value}
                </option>
              ))}
            </select>
          </label>

          <label className="flex items-center gap-2 text-sm text-zinc-600 dark:text-zinc-400">
            Sort
            <select
              value={sort}
              onChange={(e) => {
                setSort(e.target.value as IssueSort);
                setPage(1);
              }}
              className={SELECT_CLASS}
            >
              {SORT_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>

          <p className="ml-auto text-sm text-zinc-500" aria-live="polite">
            {issues === null ? "Loading…" : `${visible.length} ${visible.length === 1 ? "issue" : "issues"}`}
          </p>
        </div>

        {error && (
          <p className="rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-600 dark:border-red-900/50 dark:bg-red-900/10 dark:text-red-400">
            {error}
          </p>
        )}

        <div className="overflow-x-auto rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900">
          <table className="w-full text-left text-sm">
            <caption className="sr-only">GitHub issues, {visible.length} matching the current filters</caption>
            <thead className="border-b border-zinc-200 text-xs uppercase tracking-wide text-zinc-500 dark:border-zinc-800">
              <tr>
                <th scope="col" className="px-4 py-3">#</th>
                <th scope="col" className="px-4 py-3">Title</th>
                <th scope="col" className="px-4 py-3">Labels</th>
                <th scope="col" className="px-4 py-3">Severity</th>
                <th scope="col" className="px-4 py-3">Difficulty</th>
                <th scope="col" className="px-4 py-3">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-zinc-100 dark:divide-zinc-800">
              {issues === null &&
                Array.from({ length: 5 }).map((_, index) => (
                  <tr key={`skeleton-${index}`}>
                    <td colSpan={6} className="px-4 py-3">
                      <Skeleton height={20} />
                    </td>
                  </tr>
                ))}

              {issues !== null && rows.length === 0 && (
                <tr>
                  <td colSpan={6} className="px-4 py-10 text-center text-zinc-500">
                    No issues match the current filters.
                  </td>
                </tr>
              )}

              {rows.map((issue) => (
                <tr
                  key={issue.number}
                  onClick={() => window.open(issue.url, "_blank", "noopener,noreferrer")}
                  className="cursor-pointer hover:bg-zinc-50 dark:hover:bg-zinc-800/60"
                >
                  <td className="px-4 py-3 font-mono text-xs text-zinc-500">{issue.number}</td>
                  <td className="px-4 py-3">
                    <a
                      href={issue.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      onClick={(e) => e.stopPropagation()}
                      className="inline-flex items-center gap-1.5 font-medium text-zinc-900 hover:underline dark:text-zinc-100"
                    >
                      {issue.title}
                      <ExternalLink className="h-3.5 w-3.5 shrink-0 text-zinc-400" aria-hidden="true" />
                    </a>
                    <span className="block text-xs text-zinc-500">Created {formatDate(issue.createdAt)}</span>
                  </td>
                  <td className="px-4 py-3">
                    <span className="flex flex-wrap gap-1">
                      {issue.labels.length === 0 && <span className="text-xs text-zinc-500">—</span>}
                      {issue.labels.map((label) => (
                        <span
                          key={label}
                          className="rounded-full border border-zinc-200 px-2 py-0.5 text-[11px] text-zinc-600 dark:border-zinc-700 dark:text-zinc-400"
                        >
                          {label}
                        </span>
                      ))}
                    </span>
                  </td>
                  <td className={`px-4 py-3 capitalize font-semibold ${SEVERITY_TEXT[issue.severity]}`}>
                    {issue.severity === "none" ? "—" : issue.severity}
                  </td>
                  <td className="px-4 py-3 capitalize text-zinc-600 dark:text-zinc-400">
                    {issue.difficulty === "unknown" ? "—" : issue.difficulty}
                  </td>
                  <td className="px-4 py-3 capitalize text-zinc-600 dark:text-zinc-400">{issue.status}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {totalPages > 1 && (
          <nav aria-label="Issue pagination" className="flex items-center justify-between text-sm">
            <button
              type="button"
              onClick={() => setPage(currentPage - 1)}
              disabled={currentPage <= 1}
              className="rounded-lg border border-zinc-300 px-3 py-2 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-700"
            >
              Previous
            </button>
            <span className="text-zinc-500">
              Page {currentPage} of {totalPages} · {PAGE_SIZE} per page
            </span>
            <button
              type="button"
              onClick={() => setPage(currentPage + 1)}
              disabled={currentPage >= totalPages}
              className="rounded-lg border border-zinc-300 px-3 py-2 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-700"
            >
              Next
            </button>
          </nav>
        )}
      </main>
    </>
  );
}
