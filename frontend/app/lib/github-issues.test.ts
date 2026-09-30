import { describe, it, expect } from "vitest";
import {
  difficultyFromLabels,
  filterIssues,
  normalizeIssues,
  pageCount,
  paginate,
  severityFromLabels,
  sortIssues,
  statusFromIssue,
  type GitHubIssue,
  type IssueRow,
} from "./github-issues";

function issue(overrides: Partial<GitHubIssue> = {}): GitHubIssue {
  return {
    number: 1,
    title: "An issue",
    html_url: "https://github.com/HyperSafeD/Sanctifier/issues/1",
    state: "open",
    created_at: "2026-01-01T00:00:00Z",
    labels: [],
    ...overrides,
  };
}

function row(overrides: Partial<IssueRow> = {}): IssueRow {
  return {
    number: 1,
    title: "An issue",
    url: "https://example.com/1",
    labels: [],
    severity: "none",
    difficulty: "unknown",
    status: "open",
    createdAt: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("severityFromLabels", () => {
  it("reads prefixed and bare severity labels", () => {
    expect(severityFromLabels(["severity: high"])).toBe("high");
    expect(severityFromLabels(["priority-critical"])).toBe("critical");
    expect(severityFromLabels(["low"])).toBe("low");
  });

  it("keeps the highest severity when several are present", () => {
    expect(severityFromLabels(["severity: low", "severity: critical", "medium"])).toBe("critical");
  });

  it("ignores difficulty labels that reuse severity words", () => {
    expect(severityFromLabels(["difficulty: medium"])).toBe("none");
  });

  it("falls back to none", () => {
    expect(severityFromLabels(["frontend", "good first issue"])).toBe("none");
  });
});

describe("difficultyFromLabels", () => {
  it("reads difficulty labels", () => {
    expect(difficultyFromLabels(["difficulty: easy"])).toBe("easy");
    expect(difficultyFromLabels(["difficulty: medium"])).toBe("medium");
    expect(difficultyFromLabels(["difficulty-hard"])).toBe("hard");
  });

  it("treats good first issue as easy and advanced as hard", () => {
    expect(difficultyFromLabels(["good first issue"])).toBe("easy");
    expect(difficultyFromLabels(["advanced"])).toBe("hard");
  });

  it("is unknown when no label describes difficulty", () => {
    expect(difficultyFromLabels(["severity: high", "frontend"])).toBe("unknown");
  });
});

describe("statusFromIssue", () => {
  it("reports closed issues", () => {
    expect(statusFromIssue(issue({ state: "closed" }), [])).toBe("closed");
  });

  it("reports in progress for assigned or WIP issues", () => {
    expect(statusFromIssue(issue({ assignees: [{}] }), [])).toBe("in progress");
    expect(statusFromIssue(issue(), ["in progress"])).toBe("in progress");
  });

  it("reports open otherwise", () => {
    expect(statusFromIssue(issue(), ["frontend"])).toBe("open");
  });
});

describe("normalizeIssues", () => {
  it("drops pull requests and maps label objects", () => {
    const rows = normalizeIssues([
      issue({ number: 10, labels: [{ name: "security" }, "severity: high"] }),
      issue({ number: 11, pull_request: {} }),
    ]);

    expect(rows).toHaveLength(1);
    expect(rows[0].number).toBe(10);
    expect(rows[0].labels).toEqual(["security", "severity: high"]);
    expect(rows[0].severity).toBe("high");
  });
});

describe("filterIssues", () => {
  const rows = [
    row({ number: 1, labels: ["security"], severity: "high" }),
    row({ number: 2, labels: ["frontend"], severity: "low" }),
    row({ number: 3, labels: ["tooling", "ci"], severity: "high" }),
  ];

  it("returns everything by default", () => {
    expect(filterIssues(rows)).toHaveLength(3);
  });

  it("filters by category label", () => {
    expect(filterIssues(rows, { category: "security" }).map((r) => r.number)).toEqual([1]);
    expect(filterIssues(rows, { category: "tooling" }).map((r) => r.number)).toEqual([3]);
    expect(filterIssues(rows, { category: "contract" })).toHaveLength(0);
  });

  it("filters by severity", () => {
    expect(filterIssues(rows, { severity: "high" }).map((r) => r.number)).toEqual([1, 3]);
  });

  it("combines both filters", () => {
    expect(filterIssues(rows, { category: "frontend", severity: "high" })).toHaveLength(0);
  });
});

describe("sortIssues", () => {
  const older = row({ number: 1, createdAt: "2026-01-01T00:00:00Z", severity: "low" });
  const newer = row({ number: 2, createdAt: "2026-02-01T00:00:00Z", severity: "critical" });

  it("sorts by creation date in both directions", () => {
    expect(sortIssues([older, newer], "created-desc").map((r) => r.number)).toEqual([2, 1]);
    expect(sortIssues([older, newer], "created-asc").map((r) => r.number)).toEqual([1, 2]);
  });

  it("sorts by severity, newest first within a severity", () => {
    const sameSeverityOlder = row({ number: 3, createdAt: "2025-12-01T00:00:00Z", severity: "critical" });
    expect(sortIssues([older, sameSeverityOlder, newer], "severity").map((r) => r.number)).toEqual([2, 3, 1]);
  });

  it("does not mutate the input", () => {
    const input = [older, newer];
    sortIssues(input, "created-desc");
    expect(input.map((r) => r.number)).toEqual([1, 2]);
  });
});

describe("pagination", () => {
  const rows = Array.from({ length: 120 }, (_, index) => row({ number: index + 1 }));

  it("counts pages of 50", () => {
    expect(pageCount(120)).toBe(3);
    expect(pageCount(50)).toBe(1);
    expect(pageCount(0)).toBe(1);
  });

  it("returns the requested page", () => {
    expect(paginate(rows, 1)).toHaveLength(50);
    expect(paginate(rows, 1)[0].number).toBe(1);
    expect(paginate(rows, 2)[0].number).toBe(51);
    expect(paginate(rows, 3)).toHaveLength(20);
  });

  it("clamps out-of-range pages", () => {
    expect(paginate(rows, 0)[0].number).toBe(1);
    expect(paginate(rows, 99)[0].number).toBe(101);
  });
});
