import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Analysis Terminal | Sanctifier",
  description:
    "Stream real-time Sanctifier analysis output for Soroban smart contracts.",
};

export default function TerminalLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return children;
}
