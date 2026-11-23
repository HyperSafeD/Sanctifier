import { timingSafeEqual } from "crypto";
import { METRICS_CONTENT_TYPE, renderMetrics } from "../../lib/metrics";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

function isAuthorized(request: Request): boolean {
  const token = process.env.METRICS_TOKEN?.trim();
  if (!token) return true;

  const provided = request.headers.get("authorization")?.replace(/^Bearer\s+/i, "").trim() ?? "";
  const a = Buffer.from(provided);
  const b = Buffer.from(token);
  return a.length === b.length && timingSafeEqual(a, b);
}

/**
 * GET /metrics
 *
 * Prometheus scrape endpoint (text exposition format 0.0.4). When
 * METRICS_TOKEN is set, scrapers must send `Authorization: Bearer <token>`.
 */
export async function GET(request: Request) {
  if (!isAuthorized(request)) {
    return new Response("Unauthorized\n", {
      status: 401,
      headers: { "Content-Type": "text/plain; charset=utf-8" },
    });
  }

  return new Response(renderMetrics(), {
    headers: {
      "Content-Type": METRICS_CONTENT_TYPE,
      "Cache-Control": "no-store",
    },
  });
}
