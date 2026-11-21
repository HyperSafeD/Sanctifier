import { describe, it, expect } from "vitest";
import {
  DEFAULT_CONTRACT_FILTERS,
  countPatterns,
  deriveTags,
  filterAndSortContracts,
  riskFromCounts,
  totalFindings,
  type ContractInfo,
} from "./contracts-model";

const contract = (over: Partial<ContractInfo>): ContractInfo => ({
  name: "c",
  path: "contracts/c",
  description: "",
  tags: [],
  risk: "info",
  findingCount: 0,
  linesOfCode: 0,
  ...over,
});

describe("countPatterns", () => {
  it("counts panics, unwrap/expect, unsafe blocks and raw invokes", () => {
    const src = `
      fn a() { panic!("x"); let v = opt.unwrap(); let w = r.expect("m"); }
      fn b() { unsafe { core::ptr::null::<u8>(); } }
      fn c(env: &Env) { env.invoke_contract::<()>(&id, &sym, args); }
    `;
    expect(countPatterns(src)).toEqual({ panics: 3, unsafeBlocks: 1, rawInvokes: 1 });
  });

  it("ignores comments and the #[cfg(test)] module", () => {
    const src = `
      // panic!("commented out")
      fn ok() {}
      #[cfg(test)]
      mod tests { fn t() { x.unwrap(); panic!("in tests"); } }
    `;
    expect(totalFindings(countPatterns(src))).toBe(0);
  });
});

describe("riskFromCounts", () => {
  it("maps weighted counts onto risk levels", () => {
    expect(riskFromCounts({ panics: 0, unsafeBlocks: 0, rawInvokes: 0 })).toBe("info");
    expect(riskFromCounts({ panics: 1, unsafeBlocks: 0, rawInvokes: 0 })).toBe("low");
    expect(riskFromCounts({ panics: 4, unsafeBlocks: 0, rawInvokes: 0 })).toBe("medium");
    expect(riskFromCounts({ panics: 0, unsafeBlocks: 2, rawInvokes: 0 })).toBe("high");
    expect(riskFromCounts({ panics: 0, unsafeBlocks: 2, rawInvokes: 2 })).toBe("critical");
  });
});

describe("deriveTags", () => {
  it("derives tags from the name and description and appends extras once", () => {
    const tags = deriveTags("zk-verifier", "Groth16 proof verifier", ["tested", "tested"]);
    expect(tags).toContain("zk");
    expect(tags.filter((t) => t === "tested")).toHaveLength(1);
  });
});

describe("filterAndSortContracts", () => {
  const list = [
    contract({ name: "governance", description: "DAO voting", tags: ["governance"], risk: "low", findingCount: 2, linesOfCode: 300 }),
    contract({ name: "token-with-bugs", description: "buggy token", tags: ["token", "vulnerable"], risk: "critical", findingCount: 12, linesOfCode: 120 }),
    contract({ name: "oracle", description: "price feed", tags: ["oracle"], risk: "low", findingCount: 5, linesOfCode: 500 }),
  ];

  it("searches by name, description and tag (case-insensitive)", () => {
    const run = (query: string) =>
      filterAndSortContracts(list, { ...DEFAULT_CONTRACT_FILTERS, query }).map((c) => c.name);
    expect(run("GOVERN")).toEqual(["governance"]);
    expect(run("price")).toEqual(["oracle"]);
    expect(run("vulnerable")).toEqual(["token-with-bugs"]);
    expect(run("nothing-matches")).toEqual([]);
  });

  it("filters by risk level", () => {
    const names = filterAndSortContracts(list, { ...DEFAULT_CONTRACT_FILTERS, risk: "low" }).map((c) => c.name);
    expect(names).toEqual(["oracle", "governance"]);
  });

  it("sorts by risk (then findings), findings, and lines of code", () => {
    const sorted = (sort: "risk" | "findings" | "loc") =>
      filterAndSortContracts(list, { ...DEFAULT_CONTRACT_FILTERS, sort }).map((c) => c.name);
    expect(sorted("risk")).toEqual(["token-with-bugs", "oracle", "governance"]);
    expect(sorted("findings")).toEqual(["token-with-bugs", "oracle", "governance"]);
    expect(sorted("loc")).toEqual(["oracle", "governance", "token-with-bugs"]);
  });

  it("does not mutate the input array", () => {
    const copy = [...list];
    filterAndSortContracts(list, { ...DEFAULT_CONTRACT_FILTERS, sort: "loc" });
    expect(list).toEqual(copy);
  });
});
