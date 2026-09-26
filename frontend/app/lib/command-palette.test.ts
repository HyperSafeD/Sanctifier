import { describe, it, expect, beforeEach } from "vitest";
import {
  FINDING_CODE_ITEMS,
  MAX_RECENT,
  PAGE_ITEMS,
  SETTING_ITEMS,
  contractItem,
  pushRecentId,
  readRecentIds,
  scoreItem,
  searchItems,
} from "./command-palette";

const ALL = [...PAGE_ITEMS, ...SETTING_ITEMS, contractItem("governance", "DAO voting"), ...FINDING_CODE_ITEMS];

describe("searchItems", () => {
  it("returns exact and prefix label matches first", () => {
    expect(searchItems(ALL, "scan")[0].id).toBe("page:scan");
    expect(searchItems(ALL, "gov")[0].id).toBe("contract:governance");
  });

  it("finds finding codes and settings", () => {
    expect(searchItems(ALL, "S001")[0].id).toBe("code:S001");
    expect(searchItems(ALL, "theme")[0].action).toBe("toggle-theme");
  });

  it("matches on descriptions and returns nothing for gibberish", () => {
    expect(searchItems(ALL, "require_auth").map((i) => i.id)).toContain("code:S001");
    expect(searchItems(ALL, "zzzzqqq")).toEqual([]);
  });

  it("scores every item for an empty query", () => {
    expect(scoreItem(PAGE_ITEMS[0], "  ")).toBeGreaterThan(0);
  });
});

describe("recent items", () => {
  beforeEach(() => window.localStorage.clear());

  it("stores most-recent first without duplicates", () => {
    pushRecentId("page:scan");
    pushRecentId("page:audit");
    expect(pushRecentId("page:scan")).toEqual(["page:scan", "page:audit"]);
    expect(readRecentIds()).toEqual(["page:scan", "page:audit"]);
  });

  it("caps the list length", () => {
    for (let i = 0; i < MAX_RECENT + 3; i++) pushRecentId(`id:${i}`);
    expect(readRecentIds()).toHaveLength(MAX_RECENT);
  });

  it("ignores corrupted storage", () => {
    window.localStorage.setItem("sanctifier_palette_recent", "{not json");
    expect(readRecentIds()).toEqual([]);
  });
});
