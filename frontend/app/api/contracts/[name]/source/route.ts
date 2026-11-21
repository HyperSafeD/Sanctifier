import { loadContractSource } from "../../../../lib/contracts-catalog";
import { withMetrics } from "../../../../../lib/metrics";

export const runtime = "nodejs";

async function handleGET(_request: Request, { params }: { params: Promise<{ name: string }> }) {
  const { name } = await params;
  const source = await loadContractSource(name);

  if (source === null) {
    return Response.json({ error: "Contract not found" }, { status: 404 });
  }
  return Response.json({ name, source });
}

export const GET = withMetrics("/api/contracts/[name]/source", "GET", handleGET);
