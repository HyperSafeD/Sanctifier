import { loadContracts } from "../../lib/contracts-catalog";

export const runtime = "nodejs";

export async function GET() {
  const contracts = await loadContracts();
  return Response.json({ contracts, count: contracts.length });
}
