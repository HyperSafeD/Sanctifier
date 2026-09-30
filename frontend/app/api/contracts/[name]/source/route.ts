import { loadContractSource } from "../../../../lib/contracts-catalog";

export const runtime = "nodejs";

export async function GET(_request: Request, { params }: { params: Promise<{ name: string }> }) {
  const { name } = await params;
  const source = await loadContractSource(name);

  if (source === null) {
    return Response.json({ error: "Contract not found" }, { status: 404 });
  }
  return Response.json({ name, source });
}
