import { describe, it, expect, vi, afterEach } from "vitest";
import { GET, type TestnetStatusResponse } from "./route";

const RPC_URL = "https://soroban-testnet.stellar.org";

function stubUpstream(rpc: () => Promise<Response>) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => {
      if (String(input) === RPC_URL) return rpc();
      return new Response(JSON.stringify({ id: "CONTRACT" }), { status: 200 });
    }),
  );
}

describe("/api/testnet-status GET - upstream failures", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("degrades to networkHealthy=false when the RPC times out", async () => {
    stubUpstream(() =>
      Promise.reject(new DOMException("The operation timed out.", "TimeoutError")),
    );

    const res = await GET();
    expect(res.status).toBe(200);

    const body = (await res.json()) as TestnetStatusResponse;
    expect(body.networkHealthy).toBe(false);
    expect(body.ledger).toBeUndefined();
    expect(body.contracts).toHaveLength(3);
    expect(body.contracts.every((c) => c.alive)).toBe(true);
  });

  it("degrades to networkHealthy=false when the RPC returns malformed JSON", async () => {
    stubUpstream(async () => new Response("<html>502 Bad Gateway</html>", { status: 200 }));

    const res = await GET();
    expect(res.status).toBe(200);

    const body = (await res.json()) as TestnetStatusResponse;
    expect(body.networkHealthy).toBe(false);
    expect(body.ledger).toBeUndefined();
    expect(body.contracts).toHaveLength(3);
  });

  it("reports every contract as not alive when all upstreams fail", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new Error("ECONNREFUSED"))));

    const res = await GET();
    expect(res.status).toBe(200);

    const body = (await res.json()) as TestnetStatusResponse;
    expect(body.networkHealthy).toBe(false);
    expect(body.contracts.every((c) => !c.alive && c.errorMessage)).toBe(true);
  });
});
