import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TestnetStatusWidget } from "./TestnetStatusWidget";
import type { TestnetStatusResponse } from "../api/testnet-status/route";

const LIVE_RESPONSE: TestnetStatusResponse = {
  networkHealthy: true,
  ledger: 12345678,
  contracts: [
    {
      id: "runtime-guard-wrapper",
      label: "Runtime Guard Wrapper",
      address: "CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB",
      alive: true,
      explorerUrl:
        "https://stellar.expert/explorer/testnet/contract/CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB",
    },
    {
      id: "vulnerable-contract",
      label: "Vulnerable Contract (demo)",
      address: "CABBT5FKG7AE7IEEA4KR2J5AVYRSZAWKTXZ2KFX3UNJQAMMLMCXNLMIB",
      alive: true,
      explorerUrl:
        "https://stellar.expert/explorer/testnet/contract/CABBT5FKG7AE7IEEA4KR2J5AVYRSZAWKTXZ2KFX3UNJQAMMLMCXNLMIB",
    },
    {
      id: "reentrancy-guard",
      label: "Reentrancy Guard",
      address: "CDDVM5A5IVDAG5FZ2OU2CLWAHC7A2T7LHQHZSDVKZPE6SDMDO2JCR3UY",
      alive: true,
      explorerUrl:
        "https://stellar.expert/explorer/testnet/contract/CDDVM5A5IVDAG5FZ2OU2CLWAHC7A2T7LHQHZSDVKZPE6SDMDO2JCR3UY",
    },
  ],
  fetchedAt: "2026-06-26T00:00:00.000Z",
};

function mockFetch(response: TestnetStatusResponse | null, status = 200) {
  return vi.spyOn(global, "fetch").mockResolvedValue({
    ok: status >= 200 && status < 300,
    status,
    json: async () => response,
  } as Response);
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("TestnetStatusWidget – loading state", () => {
  it("renders a loading indicator before data arrives", () => {
    vi.spyOn(global, "fetch").mockReturnValue(new Promise(() => {}));
    render(<TestnetStatusWidget />);
    expect(screen.getByRole("status")).toBeInTheDocument();
  });

  it("loading indicator has an accessible label", () => {
    vi.spyOn(global, "fetch").mockReturnValue(new Promise(() => {}));
    render(<TestnetStatusWidget />);
    expect(screen.getByLabelText(/loading testnet status/i)).toBeInTheDocument();
  });
});

describe("TestnetStatusWidget – success state", () => {
  it("renders all three contract labels after successful fetch", async () => {
    mockFetch(LIVE_RESPONSE);
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      expect(screen.getByText("Runtime Guard Wrapper")).toBeInTheDocument();
    });
    expect(screen.getByText("Vulnerable Contract (demo)")).toBeInTheDocument();
    expect(screen.getByText("Reentrancy Guard")).toBeInTheDocument();
  });

  it("shows Live badge for each online contract", async () => {
    mockFetch(LIVE_RESPONSE);
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      const badges = screen.getAllByText("Live");
      expect(badges).toHaveLength(3);
    });
  });

  it("renders the current ledger number", async () => {
    mockFetch(LIVE_RESPONSE);
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      expect(screen.getByText(/12,345,678/)).toBeInTheDocument();
    });
  });

  it("renders explorer links for each contract", async () => {
    mockFetch(LIVE_RESPONSE);
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      const links = screen.getAllByRole("link", { name: /view .* on stellar expert/i });
      expect(links).toHaveLength(3);
    });
  });

  it("explorer links point to stellar.expert testnet", async () => {
    mockFetch(LIVE_RESPONSE);
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      const links = screen.getAllByRole("link", { name: /view .* on stellar expert/i });
      links.forEach((link) => {
        expect(link).toHaveAttribute(
          "href",
          expect.stringContaining("stellar.expert/explorer/testnet"),
        );
      });
    });
  });

  it("shows last-updated time after fetch completes", async () => {
    mockFetch(LIVE_RESPONSE);
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      expect(screen.getByText(/last updated/i)).toBeInTheDocument();
    });
  });
});

describe("TestnetStatusWidget – offline contract", () => {
  it("shows Offline badge for a contract that is down", async () => {
    const offlineResponse: TestnetStatusResponse = {
      ...LIVE_RESPONSE,
      contracts: [
        { ...LIVE_RESPONSE.contracts[0], alive: false, errorMessage: "Contract not found" },
        ...LIVE_RESPONSE.contracts.slice(1),
      ],
    };
    mockFetch(offlineResponse);
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      expect(screen.getByText("Offline")).toBeInTheDocument();
    });
  });

  it("renders the error message returned by the API", async () => {
    const offlineResponse: TestnetStatusResponse = {
      ...LIVE_RESPONSE,
      contracts: [
        {
          ...LIVE_RESPONSE.contracts[0],
          alive: false,
          errorMessage: "Contract not found on testnet",
        },
        ...LIVE_RESPONSE.contracts.slice(1),
      ],
    };
    mockFetch(offlineResponse);
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      expect(screen.getByText("Contract not found on testnet")).toBeInTheDocument();
    });
  });
});

describe("TestnetStatusWidget – network error", () => {
  it("shows an error message when the API call fails", async () => {
    vi.spyOn(global, "fetch").mockRejectedValue(new Error("Network error"));
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
  });

  it("shows the error message text", async () => {
    vi.spyOn(global, "fetch").mockRejectedValue(new Error("Network error"));
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      expect(screen.getByText(/network error/i)).toBeInTheDocument();
    });
  });

  it("shows an error when the API returns HTTP 500", async () => {
    mockFetch(null, 500);
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
  });
});

describe("TestnetStatusWidget – refresh button", () => {
  it("calls fetch again when Refresh is clicked", async () => {
    const spy = mockFetch(LIVE_RESPONSE);
    render(<TestnetStatusWidget />);
    await waitFor(() => screen.getByText(/refresh/i));

    await userEvent.click(screen.getByRole("button", { name: /refresh/i }));
    expect(spy).toHaveBeenCalledTimes(2);
  });
});

describe("TestnetStatusWidget – accessibility", () => {
  it("section has an accessible name", () => {
    vi.spyOn(global, "fetch").mockReturnValue(new Promise(() => {}));
    render(<TestnetStatusWidget />);
    expect(
      screen.getByRole("region", { name: /testnet contract status/i }),
    ).toBeInTheDocument();
  });

  it("online status dots have aria-label Online", async () => {
    mockFetch(LIVE_RESPONSE);
    render(<TestnetStatusWidget />);
    await waitFor(() => {
      const dots = screen.getAllByLabelText("Online");
      expect(dots.length).toBeGreaterThan(0);
    });
  });
});
