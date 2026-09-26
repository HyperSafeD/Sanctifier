export interface Crumb {
  label: string;
  href: string;
}

const SEGMENT_LABELS: Record<string, string> = {
  contracts: "Contracts",
  dashboard: "Dashboard",
  audit: "Audit Report",
  scan: "Scan",
  playground: "Playground",
  terminal: "Terminal",
  settings: "Settings",
  webhooks: "Webhooks",
  zk: "ZK",
  terms: "Terms of Service",
  privacy: "Privacy Policy",
};

function labelFor(segment: string): string {
  let decoded = segment;
  try {
    decoded = decodeURIComponent(segment);
  } catch {
    // keep the raw segment when it is not valid percent-encoding
  }
  // Known route segments get a friendly label; dynamic ones (e.g. a contract name) stay verbatim.
  return SEGMENT_LABELS[decoded] ?? decoded;
}

/** Build a breadcrumb trail (Home → … → current page) from a URL pathname. */
export function buildBreadcrumbs(pathname: string, homeLabel = "Home"): Crumb[] {
  const segments = pathname.split("?")[0].split("#")[0].split("/").filter(Boolean);
  const crumbs: Crumb[] = [{ label: homeLabel, href: "/" }];

  let href = "";
  for (const segment of segments) {
    href += `/${segment}`;
    crumbs.push({ label: labelFor(segment), href });
  }
  return crumbs;
}
