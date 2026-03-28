import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Security Dashboard | Sanctifier",
  description:
    "Visualize and explore Soroban smart contract security analysis findings, call graphs, and sanctity scores.",
};

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return children;
}
