/**
 * Normalization and filtering helpers for the Issue Tracker page.
 *
 * GitHub has no first-class notion of severity or difficulty, so both are
 * derived from label names (e.g. `severity: high`, `difficulty: easy`,
 * `good first issue`).
 */

export const SEVERITIES = ["critical", "high", "medium", "low", "none"] as const;
export type IssueSeverity = (typeof SEVERITIES)[number];

export type IssueDifficulty = "easy" | "medium" | "hard" | "unknown";
export type IssueStatus = "open" | "in progress" | "closed";

export const ISSUE_CATEGORIES = ["security", "frontend", "contract", "tooling"] as const;
export type IssueCategory = (typeof ISSUE_CATEGORIES)[number];

/** Label fragments that place an issue in a category. */
const CATEGORY_ALIASES: Record<IssueCategory, string[]> = {
  security: ["security", "audit", "vulnerability"],
  frontend: ["frontend", "front-end", "ui", "ux"],
  contract: ["contract", "soroban"],
  tooling: ["tooling", "tools", "ci", "build"],
};

export type IssueSort = "created-desc" | "created-asc" | "severity";

export const PAGE_SIZE = 50;

/** The subset of the GitHub REST issue payload this page relies on. */
export interface GitHubIssue {
  number: number;
  title: string;
  html_url: string;
  state: string;
  created_at: string;
  labels?: Array<{ name?: string } | string>;
  assignees?: unknown[];
  /** Present when the entry is a pull request rather than an issue. */
  pull_request?: unknown;
}

export interface IssueRow {
  number: number;
  title: string;
  url: string;
  labels: string[];
  severity: IssueSeverity;
  difficulty: IssueDifficulty;
  status: IssueStatus;
  createdAt: string;
}

function labelTokens(label: string): string[] {
  return label.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean);
}

export function labelNames(issue: GitHubIssue): string[] {
  return (issue.labels ?? [])
    .map((label) => (typeof label === "string" ? label : label?.name ?? ""))
    .filter(Boolean);
}

/** Highest severity mentioned by any label, or `"none"`. */
export function severityFromLabels(labels: string[]): IssueSeverity {
  let best: IssueSeverity = "none";
  for (const label of labels) {
    const tokens = labelTokens(label);
    if (tokens[0] === "difficulty") continue;
    for (const severity of SEVERITIES) {
      if (severity === "none" || !tokens.includes(severity)) continue;
      if (SEVERITIES.indexOf(severity) < SEVERITIES.indexOf(best)) best = severity;
    }
  }
  return best;
}

export function difficultyFromLabels(labels: string[]): IssueDifficulty {
  for (const label of labels) {
    const tokens = labelTokens(label);
    const joined = tokens.join(" ");
    if (joined === "good first issue" || joined === "easy" || tokens.includes("beginner")) return "easy";
    if (tokens.includes("advanced")) return "hard";
    if (tokens[0] !== "difficulty") continue;
    if (tokens.includes("easy") || tokens.includes("trivial")) return "easy";
    if (tokens.includes("hard")) return "hard";
    if (tokens.includes("medium")) return "medium";
  }
  return "unknown";
}

export function statusFromIssue(issue: GitHubIssue, labels: string[]): IssueStatus {
  if (issue.state === "closed") return "closed";
  const inProgress = labels.some((label) => {
    const joined = labelTokens(label).join(" ");
    return joined.includes("in progress") || joined.includes("wip");
  });
  if (inProgress || (issue.assignees?.length ?? 0) > 0) return "in progress";
  return "open";
}

export function normalizeIssue(issue: GitHubIssue): IssueRow {
  const labels = labelNames(issue);
  return {
    number: issue.number,
    title: issue.title,
    url: issue.html_url,
    labels,
    severity: severityFromLabels(labels),
    difficulty: difficultyFromLabels(labels),
    status: statusFromIssue(issue, labels),
    createdAt: issue.created_at,
  };
}

/** Normalize a GitHub listing, dropping pull requests (the API returns them too). */
export function normalizeIssues(issues: GitHubIssue[]): IssueRow[] {
  return issues.filter((issue) => !issue.pull_request).map(normalizeIssue);
}

export function matchesCategory(row: IssueRow, category: IssueCategory): boolean {
  const aliases = CATEGORY_ALIASES[category];
  return row.labels.some((label) => {
    const name = label.toLowerCase();
    return aliases.some((alias) => name.includes(alias));
  });
}

export interface IssueFilters {
  category?: IssueCategory | "all";
  severity?: IssueSeverity | "all";
}

export function filterIssues(rows: IssueRow[], filters: IssueFilters = {}): IssueRow[] {
  const { category = "all", severity = "all" } = filters;
  return rows.filter((row) => {
    if (category !== "all" && !matchesCategory(row, category)) return false;
    if (severity !== "all" && row.severity !== severity) return false;
    return true;
  });
}

export function sortIssues(rows: IssueRow[], sort: IssueSort): IssueRow[] {
  const byCreatedDesc = (a: IssueRow, b: IssueRow) =>
    new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime();

  const sorted = [...rows];
  if (sort === "created-asc") return sorted.sort((a, b) => byCreatedDesc(b, a));
  if (sort === "severity") {
    return sorted.sort(
      (a, b) => SEVERITIES.indexOf(a.severity) - SEVERITIES.indexOf(b.severity) || byCreatedDesc(a, b),
    );
  }
  return sorted.sort(byCreatedDesc);
}

export function pageCount(total: number, size = PAGE_SIZE): number {
  return Math.max(1, Math.ceil(total / size));
}

/** One page of rows; `page` is 1-based and clamped to the available range. */
export function paginate<T>(rows: T[], page: number, size = PAGE_SIZE): T[] {
  const safePage = Math.min(Math.max(1, Math.trunc(page) || 1), pageCount(rows.length, size));
  const start = (safePage - 1) * size;
  return rows.slice(start, start + size);
}
