import { loadContracts } from "../../lib/contracts-catalog";
import { withMetrics } from "../../../lib/metrics";

export const runtime = "nodejs";

async function handleGET() {
  const contracts = await loadContracts();
  return Response.json({ contracts, count: contracts.length });
}

export const GET = withMetrics("/api/contracts", "GET", handleGET);
