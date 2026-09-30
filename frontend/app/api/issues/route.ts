import { normalizeIssues, type GitHubIssue } from "../../lib/github-issues";

export const runtime = "nodejs";

/** `owner/repo` whose issues the tracker displays. */
const REPO = process.env.GITHUB_REPO?.trim() || "HyperSafeD/Sanctifier";
const PER_PAGE = 100;
const MAX_PAGES = 3;
const REVALIDATE_SECONDS = 300;

export async function GET() {
  const headers: Record<string, string> = {
    Accept: "application/vnd.github+json",
    "X-GitHub-Api-Version": "2022-11-28",
  };
  // Optional — lifts the unauthenticated 60 requests/hour rate limit.
  const token = process.env.GITHUB_TOKEN?.trim();
  if (token) headers.Authorization = `Bearer ${token}`;

  const collected: GitHubIssue[] = [];

  try {
    for (let page = 1; page <= MAX_PAGES; page++) {
      const url = `https://api.github.com/repos/${REPO}/issues?state=all&per_page=${PER_PAGE}&page=${page}`;
      const response = await fetch(url, { headers, next: { revalidate: REVALIDATE_SECONDS } });

      if (!response.ok) {
        if (collected.length > 0) break;
        return Response.json(
          {
            issues: [],
            repo: REPO,
            status: "error",
            message: `GitHub API responded with ${response.status}`,
          },
          { status: 502 },
        );
      }

      const batch = (await response.json()) as GitHubIssue[];
      if (!Array.isArray(batch) || batch.length === 0) break;
      collected.push(...batch);
      if (batch.length < PER_PAGE) break;
    }

    return Response.json({ issues: normalizeIssues(collected), repo: REPO, status: "ok" });
  } catch {
    return Response.json(
      { issues: [], repo: REPO, status: "error", message: "Could not reach the GitHub API" },
      { status: 502 },
    );
  }
}
