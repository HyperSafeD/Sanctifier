import { NextRequest } from "next/server";
import { readFile } from "fs/promises";
import { existsSync } from "fs";
import path from "path";

export const runtime = "nodejs";

const RECENT_FINDINGS_FILE = path.resolve(process.cwd(), ".recent-findings.json");

let cachedFindings: unknown[] | null = null;
let cachedTimestamp: number | null = null;

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const limitParam = searchParams.get("limit");
  const limit = limitParam ? Math.min(Math.max(parseInt(limitParam, 10) || 10, 1), 100) : 10;

  try {
    let findings: unknown[];
    let timestamp: number;

    if (cachedFindings !== null && cachedTimestamp !== null) {
      findings = cachedFindings;
      timestamp = cachedTimestamp;
    } else if (existsSync(RECENT_FINDINGS_FILE)) {
      const content = await readFile(RECENT_FINDINGS_FILE, "utf-8");
      const parsed = JSON.parse(content);
      findings = Array.isArray(parsed.findings) ? parsed.findings : [];
      timestamp = parsed.timestamp ?? Date.now();
    } else {
      return Response.json({
        findings: [],
        timestamp: null,
        count: 0,
        status: "no_data",
      });
    }

    const last10 = findings.slice(0, limit);

    return Response.json({
      findings: last10,
      timestamp,
      count: last10.length,
      total_available: findings.length,
      status: "ok",
    });
  } catch {
    return Response.json({
      findings: [],
      timestamp: null,
      count: 0,
      status: "error",
    });
  }
}

export function updateRecentFindings(findings: unknown[]) {
  cachedFindings = findings;
  cachedTimestamp = Date.now();
}

export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const findings = Array.isArray(body.findings) ? body.findings : [];
    updateRecentFindings(findings);

    return Response.json({ status: "ok", count: findings.length });
  } catch {
    return Response.json({ status: "error", message: "Invalid payload" }, { status: 400 });
  }
}
