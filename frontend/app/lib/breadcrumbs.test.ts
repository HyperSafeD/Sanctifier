import { describe, it, expect } from "vitest";
import { buildBreadcrumbs } from "./breadcrumbs";

describe("buildBreadcrumbs", () => {
  it("returns only Home for the root path", () => {
    expect(buildBreadcrumbs("/")).toEqual([{ label: "Home", href: "/" }]);
  });

  it("maps known segments to friendly labels", () => {
    expect(buildBreadcrumbs("/dashboard/webhooks")).toEqual([
      { label: "Home", href: "/" },
      { label: "Dashboard", href: "/dashboard" },
      { label: "Webhooks", href: "/dashboard/webhooks" },
    ]);
  });

  it("keeps dynamic segments (e.g. a contract name) verbatim", () => {
    const crumbs = buildBreadcrumbs("/contracts/governance");
    expect(crumbs.map((c) => c.label)).toEqual(["Home", "Contracts", "governance"]);
    expect(crumbs[2].href).toBe("/contracts/governance");
  });

  it("decodes percent-encoded segments and tolerates malformed ones", () => {
    expect(buildBreadcrumbs("/contracts/my%20contract").at(-1)?.label).toBe("my contract");
    expect(buildBreadcrumbs("/contracts/100%").at(-1)?.label).toBe("100%");
  });

  it("ignores query strings, hashes and trailing slashes", () => {
    expect(buildBreadcrumbs("/scan/?contract=x#top")).toHaveLength(2);
  });
});
