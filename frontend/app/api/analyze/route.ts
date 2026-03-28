import { NextRequest } from "next/server";
import { spawn } from "child_process";
import path from "path";
import os from "os";
import { mkdtemp, rm, writeFile } from "fs/promises";
import { normalizeReport, transformReport } from "../../lib/transform";
import type { Finding } from "../../types";

export const runtime = "nodejs";

const REPO_ROOT = path.resolve(process.cwd(), "..");
const SUPPORTED_SOURCE_EXTENSIONS = new Set([".rs"]);
const MAX_FILE_SIZE_BYTES = 250 * 1024;
const EXECUTION_TIMEOUT_MS = 30000;
const RATE_LIMIT_REQUESTS_PER_MINUTE = 10;
const SANCTIFIER_BIN = process.env.SANCTIFIER_BIN?.trim() || "sanctifier";

const rateLimitMap = new Map<string, { count: number; resetTime: number }>();

function getClientIP(request: NextRequest): string {
  const xForwardedFor = request.headers.get("x-forwarded-for");
  if (xForwardedFor) {
    return xForwardedFor.split(",")[0].trim();
  }
  const xRealIP = request.headers.get("x-real-ip");
  if (xRealIP) {
    return xRealIP;
  }
  return "unknown";
}

function checkRateLimit(ip: string): { allowed: boolean; retryAfter: number } {
  const now = Date.now();
  const entry = rateLimitMap.get(ip);
  
  if (!entry || now > entry.resetTime) {
    rateLimitMap.set(ip, { count: 1, resetTime: now + 60000 });
    return { allowed: true, retryAfter: 0 };
  }
  
  if (entry.count >= RATE_LIMIT_REQUESTS_PER_MINUTE) {
    const retryAfter = Math.ceil((entry.resetTime - now) / 1000);
    return { allowed: false, retryAfter };
  }
  
  entry.count++;
  return { allowed: true, retryAfter: 0 };
}

setInterval(() => {
  const now = Date.now();
  for (const [ip, entry] of rateLimitMap.entries()) {
    if (now > entry.resetTime) {
      rateLimitMap.delete(ip);
    }
  }
}, 60000);

type ProcessResult = {
  stdout: string;
  stderr: string;
  exitCode: number | null;
};

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

    cliProcess.stdout.on("data", (data) => {
      stdout += data.toString();
    });

    cliProcess.stderr.on("data", (data) => {
      stderr += data.toString();
    });

    cliProcess.on("close", (exitCode) => {
      if (!completed) {
        completed = true;
        cleanup();
        resolve({ stdout, stderr, exitCode });
      }
    });

    cliProcess.on("error", (err) => {
      if (!completed) {
        completed = true;
        cleanup();
        reject(err);
      }
    });
  });
}

function looksLikeSorobanContract(source: string): boolean {
  return source.includes("soroban_sdk") || source.includes("soroban-sdk");
}

function parseMultipartSource(formData: FormData): Promise<{ fileName: string; source: string } | null> {
  const contract = formData.get("contract");

  if (!(contract instanceof File)) {
    return Promise.resolve(null);
  }

  if (contract.size > MAX_FILE_SIZE_BYTES) {
    throw new Error(`PAYLOAD_TOO_LARGE:${MAX_FILE_SIZE_BYTES}`);
  }

  const extension = path.extname(contract.name).toLowerCase();
  if (!SUPPORTED_SOURCE_EXTENSIONS.has(extension)) {
    throw new Error("UNSUPPORTED_EXTENSION");
  }

  return contract.arrayBuffer().then((buffer) => {
    const fileBuffer = Buffer.from(buffer);
    if (!isValidUtf8(fileBuffer)) {
      throw new Error("INVALID_UTF8");
    }

    return {
      fileName: sanitizeFileName(contract.name),
      source: fileBuffer.toString("utf8"),
    };
  });
}

function parseJsonResponse(body: string): unknown | null {
  if (!body.trim()) {
    return null;
  }

  try {
    return JSON.parse(body);
  } catch {
    return null;
  }
}

function isValidUtf8(buffer: Buffer): boolean {
  try {
    buffer.toString("utf8");
    return true;
  } catch {
    return false;
  }
}

function sanitizeFileName(name: string): string {
  const sanitized = name.replace(/[^a-zA-Z0-9._-]/g, "_");
  if (sanitized === "" || sanitized === "." || sanitized === "..") {
    return "contract.rs";
  }
  if (sanitized.startsWith(".") && sanitized.length === 1) {
    return "contract.rs";
  }
  return sanitized;
}

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const projectPath = searchParams.get("path") || ".";

  const encoder = new TextEncoder();
  const stream = new ReadableStream({
    start(controller) {
      const cliProcess = spawn(
        "cargo",
        ["run", "--quiet", "--bin", "sanctifier", "--", "analyze", projectPath],
        {
          cwd: REPO_ROOT,
          env: { ...process.env, FORCE_COLOR: "0" },
        }
      );

      const sendLog = (data: string) => {
        const lines = data.split("\n");
        for (const line of lines) {
          if (line.trim()) {
            controller.enqueue(encoder.encode(`data: ${JSON.stringify(line)}\n\n`));
          }
        }
      };

      cliProcess.stdout.on("data", (data) => {
        sendLog(data.toString());
      });

      cliProcess.stderr.on("data", (data) => {
        sendLog(`[DEBUG] ${data.toString()}`);
      });

      cliProcess.on("close", (code) => {
        controller.enqueue(
          encoder.encode(
            `data: ${JSON.stringify(
              `--- Analysis complete with exit code ${code} ---`
            )}\n\n`
          )
        );
        controller.close();
      });

      cliProcess.on("error", (err) => {
        controller.enqueue(
          encoder.encode(
            `data: ${JSON.stringify(`Error spawning process: ${err.message}`)}\n\n`
          )
        );
        controller.close();
      });
    },
  });

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache",
      Connection: "keep-alive",
    },
  });
}

export async function POST(request: NextRequest) {
  const clientIP = getClientIP(request);
  const rateLimitResult = checkRateLimit(clientIP);
  
  if (!rateLimitResult.allowed) {
    return Response.json(
      { error: "Rate limit exceeded. Please try again later." },
      { 
        status: 429,
        headers: { "Retry-After": rateLimitResult.retryAfter.toString() }
      }
    );
  }

  const tempDir = await mkdtemp(path.join(os.tmpdir(), "sanctifier-contract-"));

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
        return Response.json({ error: "Provide JSON body as { source: string }." }, { status: 400 });
      }

      if (Buffer.byteLength(source, "utf8") > MAX_FILE_SIZE_BYTES) {
        return Response.json(
          { error: `Source exceeds limit of ${MAX_FILE_SIZE_BYTES / 1024} KB.` },
          { status: 413 }
        );
      }

      sourcePayload = { fileName: "contract.rs", source };
    } else if (contentType.includes("multipart/form-data")) {
      const formData = await request.formData();
      sourcePayload = await parseMultipartSource(formData);
      if (!sourcePayload) {
        return Response.json({ error: "Attach a Rust .rs file in `contract` field." }, { status: 400 });
      }
    } else {
      return Response.json(
        { error: "Content-Type must be multipart/form-data or application/json." },
        { status: 400 }
      );
    }

    if (!looksLikeSorobanContract(sourcePayload.source)) {
      return Response.json(
        { error: "Source is not a Soroban contract (missing soroban-sdk import)." },
        { status: 422 }
      );
    }

    const contractPath = path.join(tempDir, sourcePayload.fileName);
    await writeFile(contractPath, sourcePayload.source, "utf8");

    const { stdout, stderr, exitCode } = await runAnalyzeCommand(contractPath, EXECUTION_TIMEOUT_MS);
    const report = parseJsonResponse(stdout);

    if (report) {
      const findings: Finding[] = transformReport(normalizeReport(report));
      return Response.json(findings);
    }

    return Response.json(
      {
        error:
          stderr.trim() ||
          stdout.trim() ||
          `Contract analysis failed with exit code ${exitCode ?? "unknown"}.`,
      },
      { status: 500 }
    );
  } catch (error) {
    if (error instanceof Error && error.message.startsWith("PAYLOAD_TOO_LARGE:")) {
      return Response.json(
        { error: `File size exceeds limit of ${MAX_FILE_SIZE_BYTES / 1024} KB.` },
        { status: 413 }
      );
    }
    if (error instanceof Error && error.message === "UNSUPPORTED_EXTENSION") {
      return Response.json(
        { error: "Only .rs contract source files are supported right now." },
        { status: 400 }
      );
    }
    if (error instanceof Error && error.message === "INVALID_UTF8") {
      return Response.json({ error: "File content is not valid UTF-8." }, { status: 400 });
    }
    if (error instanceof Error && error.message.includes("timed out")) {
      return Response.json(
        { error: "Analysis timed out. Please try with a smaller contract." },
        { status: 504 }
      );
    }
    return Response.json(
      {
        error:
          error instanceof Error ? error.message : "Contract analysis failed unexpectedly.",
      },
      { status: 500 }
    );
  } finally {
    await rm(tempDir, { recursive: true, force: true }).catch(() => {});
  }
}
