import { NextResponse } from "next/server";

const SOROBAN_RPC = "https://soroban-testnet.stellar.org";
const STELLAR_EXPERT_BASE =
  "https://api.stellar.expert/explorer/testnet/contract";

const CONTRACTS = [
  {
    id: "runtime-guard-wrapper",
    label: "Runtime Guard Wrapper",
    address: "CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB",
  },
  {
    id: "vulnerable-contract",
    label: "Vulnerable Contract (demo)",
    address: "CABBT5FKG7AE7IEEA4KR2J5AVYRSZAWKTXZ2KFX3UNJQAMMLMCXNLMIB",
  },
  {
    id: "reentrancy-guard",
    label: "Reentrancy Guard",
    address: "CDDVM5A5IVDAG5FZ2OU2CLWAHC7A2T7LHQHZSDVKZPE6SDMDO2JCR3UY",
  },
] as const;

export type ContractStatus = {
  id: string;
  label: string;
  address: string;
  alive: boolean;
  explorerUrl: string;
  errorMessage?: string;
};

export type TestnetStatusResponse = {
  networkHealthy: boolean;
  ledger?: number;
  contracts: ContractStatus[];
  fetchedAt: string;
};

async function getRpcHealth(): Promise<{ healthy: boolean; ledger?: number }> {
  const body = JSON.stringify({
    jsonrpc: "2.0",
    id: 1,
    method: "getLatestLedger",
    params: {},
  });

  const res = await fetch(SOROBAN_RPC, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
    signal: AbortSignal.timeout(8_000),
  });

  if (!res.ok) return { healthy: false };

  const json = (await res.json()) as {
    result?: { sequence?: number };
    error?: unknown;
  };

  if (json.error || !json.result) return { healthy: false };
  return { healthy: true, ledger: json.result.sequence };
}

async function getContractInfo(
  address: string,
): Promise<{ alive: boolean; errorMessage?: string }> {
  const url = `${STELLAR_EXPERT_BASE}/${address}`;
  try {
    const res = await fetch(url, {
      headers: { Accept: "application/json" },
      signal: AbortSignal.timeout(8_000),
    });
    if (res.status === 404) {
      return { alive: false, errorMessage: "Contract not found on testnet" };
    }
    if (!res.ok) {
      return {
        alive: false,
        errorMessage: `HTTP ${res.status} from Stellar Expert`,
      };
    }
    const json = (await res.json()) as { id?: string };
    return { alive: Boolean(json.id) };
  } catch (err) {
    return {
      alive: false,
      errorMessage: err instanceof Error ? err.message : "Fetch error",
    };
  }
}

export async function GET(): Promise<NextResponse<TestnetStatusResponse>> {
  const [networkResult, ...contractResults] = await Promise.allSettled([
    getRpcHealth(),
    ...CONTRACTS.map((c) => getContractInfo(c.address)),
  ]);

  const { healthy: networkHealthy, ledger } =
    networkResult.status === "fulfilled"
      ? networkResult.value
      : { healthy: false, ledger: undefined };

  const contracts: ContractStatus[] = CONTRACTS.map((c, i) => {
    const result = contractResults[i];
    const { alive, errorMessage } =
      result?.status === "fulfilled"
        ? result.value
        : { alive: false, errorMessage: "Request failed" };

    return {
      id: c.id,
      label: c.label,
      address: c.address,
      alive,
      explorerUrl: `https://stellar.expert/explorer/testnet/contract/${c.address}`,
      ...(errorMessage ? { errorMessage } : {}),
    };
  });

  return NextResponse.json(
    { networkHealthy, ledger, contracts, fetchedAt: new Date().toISOString() },
    {
      headers: {
        "Cache-Control": "public, s-maxage=30, stale-while-revalidate=60",
      },
    },
  );
}
