import { type NextRequest } from "next/server";
import { spawn } from "child_process";
import { SANCTIFIER_BIN } from "../../../lib/env";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

/**
 * GET /api/analyze/stream?path=<contract-path>
 *
 * Spawns `sanctifier analyze --format json` and forwards each output line
 * as a Server-Sent Event so /terminal can consume it via EventSource.
 *
 * Each SSE event has the shape:
 *   data: {"type":"log","message":"..."}


 *   data: {"type":"result","payload":{...}}


 *   data: {"type":"done"}


 */
export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const contractPath = searchParams.get("path") ?? ".";

  const encoder = new TextEncoder();

  const stream = new ReadableStream({
    start(controller) {
      function send(obj: Record<string, unknown>) {
        controller.enqueue(encoder.encode(`data: ${JSON.stringify(obj)}

`));
      }

      const bin = SANCTIFIER_BIN ?? "sanctifier";
      const child = spawn(bin, ["analyze", contractPath, "--format", "json"], {
        stdio: ["ignore", "pipe", "pipe"],
      });

      let buffer = "";

      child.stdout.on("data", (chunk: Buffer) => {
        buffer += chunk.toString();
        const lines = buffer.split("
");
        buffer = lines.pop() ?? "";
        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed) continue;
          try {
            const parsed = JSON.parse(trimmed);
            send({ type: "result", payload: parsed });
          } catch {
            send({ type: "log", message: trimmed });
          }
        }
      });

      child.stderr.on("data", (chunk: Buffer) => {
        send({ type: "log", message: chunk.toString().trim() });
      });

      child.on("close", (code) => {
        if (buffer.trim()) {
          try {
            send({ type: "result", payload: JSON.parse(buffer.trim()) });
          } catch {
            send({ type: "log", message: buffer.trim() });
          }
        }
        send({ type: "done", exitCode: code ?? 0 });
        controller.close();
      });

      child.on("error", (err) => {
        send({ type: "error", message: err.message });
        controller.close();
      });

      // Abort when client disconnects
      request.signal.addEventListener("abort", () => {
        child.kill();
        controller.close();
      });
    },
  });

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache, no-transform",
      Connection: "keep-alive",
      "X-Accel-Buffering": "no",
    },
  });
}
