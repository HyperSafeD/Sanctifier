import { NextRequest } from "next/server";
import { spawn } from "child_process";
import path from "path";
import os from "os";
import { mkdtemp, rm, writeFile } from "fs/promises";
import { normalizeReport, transformReport } from "../../../lib/transform";
import { findingsToSarif } from "../../../lib/sarif";
import { updateRecentFindings } from "../../recent-findings/route";
import {
  SANCTIFIER_BIN,
  API_KEYS,
  API_RATE_LIMIT_PER_MINUTE,
} from "../../../lib/env";

export const runtime = "nodejs";

const REPO_ROOT = path.resolve(process.cwd(), "..");
const MAX_FILE_SIZE_BYTES = 250 * 1024;
const EXECUTION_TIMEOUT_MS = 30000;

const rateLimitMap = new Map<string, { count: number; resetTime: number }>();

setInterval(() => {
  const now = Date.now();
  for (const [key, entry] of rateLimitMap.entries()) {
    if (now > entry.resetTime) {
      rateLimitMap.delete(key);
    }
  }
}, 60000);

function validateApiKey(request: NextRequest): string | null {
  const authHeader = request.headers.get("x-api-key") || request.headers.get("authorization")?.replace(/^Bearer\s+/i, "");
  if (!authHeader) return null;
  const key = authHeader.trim();
  if (API_KEYS.length === 0) {
    return key; // Accept any key if none configured
  }
  return API_KEYS.includes(key) ? key : null;
}

function checkRateLimit(apiKey: string): { allowed: boolean; retryAfter: number } {
  const now = Date.now();
  const entry = rateLimitMap.get(apiKey);

  if (!entry || now > entry.resetTime) {
    rateLimitMap.set(apiKey, { count: 1, resetTime: now + 60000 });
    return { allowed: true, retryAfter: 0 };
  }

  if (entry.count >= API_RATE_LIMIT_PER_MINUTE) {
    const retryAfter = Math.ceil((entry.resetTime - now) / 1000);
    return { allowed: false, retryAfter };
  }

  entry.count++;
  return { allowed: true, retryAfter: 0 };
}

interface ProcessResult {
  stdout: string;
  stderr: string;
  exitCode: number | null;
}

function runAnalyzeCommand(contractPath: string, timeoutMs: number): Promise<ProcessResult> {
  return new Promise((resolve, reject) => {
    const cliProcess = spawn(
      SANCTIFIER_BIN,
      ["analyze", "--format", "json", contractPath],
      {
        cwd: REPO_ROOT,
        env: { ...process.env, FORCE_COLOR: "0" },
      }
    );
    let stdout = "";
    let stderr = "";
    let timeoutId: NodeJS.Timeout | null = null;
    let completed = false;

    const cleanup = () => {
      if (timeoutId) {
        clearTimeout(timeoutId);
      }
    };

    timeoutId = setTimeout(() => {
      if (!completed) {
        completed = true;
        cleanup();
        cliProcess.kill("SIGTERM");
        reject(new Error(`Analysis timed out after ${timeoutMs / 1000} seconds`));
      }
    }, timeoutMs);

    cliProcess.stdout.on("data", (data: Buffer) => {
      stdout += data.toString();
    });

    cliProcess.stderr.on("data", (data: Buffer) => {
      stderr += data.toString();
    });

    cliProcess.on("close", (exitCode: number | null) => {
      if (!completed) {
        completed = true;
        cleanup();
        resolve({ stdout, stderr, exitCode });
      }
    });

    cliProcess.on("error", (err: Error) => {
      if (!completed) {
        completed = true;
        cleanup();
        reject(err);
      }
    });
  });
}

function jsonResponse(data: unknown, status = 200, headers?: Record<string, string>) {
  return Response.json(data, {
    status,
    headers: {
      "x-sanctifier-version": "1.0.0",
      ...headers,
    },
  });
}

export async function POST(request: NextRequest) {
  // Validate API key
  const apiKey = validateApiKey(request);
  if (!apiKey) {
    return jsonResponse(
      { error: "Unauthorized. Provide a valid API key via x-api-key header." },
      401
    );
  }

  // Rate limit
  const rateLimitResult = checkRateLimit(apiKey);
  if (!rateLimitResult.allowed) {
    return jsonResponse(
      { error: "Rate limit exceeded. Please try again later." },
      429,
      { "retry-after": rateLimitResult.retryAfter.toString() }
    );
  }

  // Determine output format
  const format = request.nextUrl.searchParams.get("format") || "json";
  if (format !== "json" && format !== "sarif") {
    return jsonResponse(
      { error: "Invalid format. Use 'json' or 'sarif'." },
      400
    );
  }

  const tempDir = await mkdtemp(path.join(os.tmpdir(), "sanctifier-v1-"));
  try {
    const contentType = request.headers.get("content-type") ?? "";

    let sourcePayload: { fileName: string; source: string } | null = null;

    if (contentType.includes("application/json")) {
      const body = await request.json().catch(() => null);
      const source =
        body && typeof body === "object" && "source" in body && typeof body.source === "string"
          ? body.source
          : null;

      if (!source || !source.trim()) {
        return jsonResponse({ error: "Provide JSON body as { source: string }." }, 400);
      }

      if (Buffer.byteLength(source, "utf8") > MAX_FILE_SIZE_BYTES) {
        return jsonResponse(
          { error: `Source exceeds limit of ${MAX_FILE_SIZE_BYTES / 1024} KB.` },
          413
        );
      }

      sourcePayload = { fileName: "contract.rs", source };
    } else if (contentType.includes("multipart/form-data")) {
      const formData = await request.formData();
      const contract = formData.get("contract");

      if (!(contract instanceof File)) {
        return jsonResponse({ error: "Attach a Rust .rs file in `contract` field." }, 400);
      }

      if (contract.size > MAX_FILE_SIZE_BYTES) {
        return jsonResponse(
          { error: `File size exceeds limit of ${MAX_FILE_SIZE_BYTES / 1024} KB.` },
          413
        );
      }

      const extension = path.extname(contract.name).toLowerCase();
      if (extension !== ".rs") {
        return jsonResponse({ error: "Only .rs contract source files are supported." }, 400);
      }

      const buffer = Buffer.from(await contract.arrayBuffer());
      sourcePayload = {
        fileName: contract.name.replace(/[^a-zA-Z0-9._-]/g, "_"),
        source: buffer.toString("utf8"),
      };
    } else {
      return jsonResponse(
        { error: "Content-Type must be multipart/form-data or application/json." },
        400
      );
    }

    // Check for Soroban contract
    if (!sourcePayload.source.includes("soroban_sdk") && !sourcePayload.source.includes("soroban-sdk")) {
      return jsonResponse(
        { error: "Source is not a Soroban contract (missing soroban-sdk import)." },
        422
      );
    }

    const contractPath = path.join(tempDir, sourcePayload.fileName);
    await writeFile(contractPath, sourcePayload.source, "utf8");

    const { stdout, stderr, exitCode } = await runAnalyzeCommand(contractPath, EXECUTION_TIMEOUT_MS);

    // Parse result
    let report: unknown = null;
    try {
      report = JSON.parse(stdout);
    } catch {
      // Not valid JSON
    }

    if (report) {
      const normalized = normalizeReport(report);
      const findings = transformReport(normalized);
      updateRecentFindings(findings);

      if (format === "sarif") {
        const sarifLog = findingsToSarif(findings);
        return jsonResponse(sarifLog);
      }

      return jsonResponse({
        success: true,
        summary: {
          total_findings: findings.length,
          critical: findings.filter((f) => f.severity === "critical").length,
          high: findings.filter((f) => f.severity === "high").length,
          medium: findings.filter((f) => f.severity === "medium").length,
          low: findings.filter((f) => f.severity === "low").length,
        },
        findings,
        report: normalized,
      });
    }

    return jsonResponse(
      {
        error: stderr.trim() || stdout.trim() || `Analysis failed with exit code ${exitCode ?? "unknown"}.`,
      },
      500
    );
  } catch (error) {
    const message = error instanceof Error ? error.message : "Analysis failed unexpectedly.";
    const status = message.includes("timed out") ? 504 : 500;
    return jsonResponse({ error: message }, status);
  } finally {
    await rm(tempDir, { recursive: true, force: true }).catch(() => {});
  }
}

export async function GET() {
  return jsonResponse({
    service: "Sanctifier Security Analysis API",
    version: "1.0.0",
    endpoints: {
      "POST /api/v1/analyze": "Submit a Soroban contract for security analysis",
    },
    auth: "x-api-key header",
    rate_limiting: `${API_RATE_LIMIT_PER_MINUTE} requests per minute per key`,
    documentation: "https://github.com/HyperSafeD/Sanctifier",
  });
}
