import { NextRequest } from "next/server";
import type { Finding } from "../../../types";
import { getExplanation, streamExplanation, getProvider } from "./providers";

export const runtime = "nodejs";

export async function POST(req: NextRequest) {
  const provider = getProvider();
  if (provider.name === "stub" && !process.env.AI_EXPLAIN_PROVIDER && process.env.STUB_AI !== "1") {
    return new Response(
      JSON.stringify({ error: "AI provider not configured", set: ["AI_EXPLAIN_PROVIDER"] }),
      { status: 503, headers: { "Content-Type": "application/json" } },
    );
  }

  try {
    const { finding, stream } = (await req.json()) as { finding: Finding; stream?: boolean };

    if (!finding) {
      return new Response(
        JSON.stringify({ error: "Finding data is required" }),
        { status: 400, headers: { "Content-Type": "application/json" } },
      );
    }

    const ip =
      req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ||
      req.headers.get("x-real-ip") ||
      "127.0.0.1";

    if (stream && provider.explainStream) {
      const encoder = new TextEncoder();
      const streamInstance = streamExplanation(finding, ip);

      const readable = new ReadableStream({
        async start(controller) {
          try {
            for await (const chunk of streamInstance) {
              controller.enqueue(encoder.encode(`data: ${JSON.stringify({ text: chunk })}\n\n`));
            }
          } catch (err: unknown) {
            const msg = err instanceof Error ? err.message : "Stream error";
            controller.enqueue(encoder.encode(`data: ${JSON.stringify({ error: msg })}\n\n`));
          } finally {
            controller.enqueue(encoder.encode("data: [DONE]\n\n"));
            controller.close();
          }
        },
      });

      return new Response(readable, {
        headers: {
          "Content-Type": "text/event-stream",
          "Cache-Control": "no-cache",
          Connection: "keep-alive",
        },
      });
    }

    const result = await getExplanation(finding, ip);
    return new Response(JSON.stringify(result), {
      headers: { "Content-Type": "application/json" },
    });
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : "Internal server error";
    const status = msg.includes("Rate limit") ? 429 : msg.includes("cost cap") ? 429 : 500;
    return new Response(JSON.stringify({ error: msg }), {
      status,
      headers: { "Content-Type": "application/json" },
    });
  }
}
