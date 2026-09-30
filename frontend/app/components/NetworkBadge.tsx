"use client";

import { useEffect, useState } from "react";

type Network = "testnet" | "futurenet" | "mainnet";

function detectNetwork(): Network {
  if (typeof window === "undefined") return "testnet";
  try {
    const stored = window.localStorage.getItem("sanctifier_network");
    if (stored === "mainnet" || stored === "futurenet" || stored === "testnet") {
      return stored;
    }
  } catch {
    // localStorage unavailable
  }
  const host = window.location.hostname;
  if (host.includes("mainnet") || host.startsWith("app")) return "mainnet";
  if (host.includes("futurenet")) return "futurenet";
  return "testnet";
}

const NETWORK_STYLES: Record<Network, { label: string; className: string }> = {
  testnet: {
    label: "TESTNET",
    className:
      "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400 border-green-300 dark:border-green-700",
  },
  futurenet: {
    label: "FUTURENET",
    className:
      "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400 border-yellow-300 dark:border-yellow-700",
  },
  mainnet: {
    label: "MAINNET",
    className:
      "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400 border-red-300 dark:border-red-700",
  },
};

export function NetworkBadge() {
  const [network, setNetwork] = useState<Network>("testnet");

  useEffect(() => {
    setNetwork(detectNetwork());
  }, []);

  const style = NETWORK_STYLES[network];

  return (
    <span
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold border ${style.className}`}
      title={`Connected to ${network}`}
    >
      {style.label}
    </span>
  );
}
