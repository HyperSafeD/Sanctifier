import { describe, it, expect, vi } from "vitest";
import {
  TAB_ROUTES,
  emitShortcut,
  isEditableTarget,
  onShortcut,
  resolveShortcut,
} from "./keyboard-shortcuts";

describe("resolveShortcut", () => {
  it("maps ⌘/Ctrl + S to save", () => {
    expect(resolveShortcut({ key: "s", metaKey: true })).toEqual({ kind: "action", action: "save" });
    expect(resolveShortcut({ key: "S", ctrlKey: true })).toEqual({ kind: "action", action: "save" });
  });

  it("maps ⌘/Ctrl + U to upload and ⌘ + Enter to run", () => {
    expect(resolveShortcut({ key: "u", ctrlKey: true })).toEqual({ kind: "action", action: "upload" });
    expect(resolveShortcut({ key: "Enter", metaKey: true })).toEqual({ kind: "action", action: "run" });
  });

  it("maps ⌘/Ctrl + 1…5 to the tab routes", () => {
    expect(resolveShortcut({ key: "1", metaKey: true })).toEqual({ kind: "navigate", href: TAB_ROUTES[0].href });
    expect(resolveShortcut({ key: "5", ctrlKey: true })).toEqual({ kind: "navigate", href: TAB_ROUTES[4].href });
    expect(resolveShortcut({ key: "6", ctrlKey: true })).toBeNull();
  });

  it("maps ? to the help modal outside text fields", () => {
    expect(resolveShortcut({ key: "?" })).toEqual({ kind: "help" });
    expect(resolveShortcut({ key: "?", isEditableTarget: true })).toBeNull();
  });

  it("ignores plain keys and modifier combinations it does not own", () => {
    expect(resolveShortcut({ key: "s" })).toBeNull();
    expect(resolveShortcut({ key: "s", metaKey: true, altKey: true })).toBeNull();
    expect(resolveShortcut({ key: "k", metaKey: true })).toBeNull();
  });
});

describe("isEditableTarget", () => {
  it("detects text entry elements", () => {
    expect(isEditableTarget(document.createElement("input"))).toBe(true);
    expect(isEditableTarget(document.createElement("textarea"))).toBe(true);
    expect(isEditableTarget(document.createElement("select"))).toBe(true);
    expect(isEditableTarget(document.createElement("div"))).toBe(false);
    expect(isEditableTarget(null)).toBe(false);
  });
});

describe("emitShortcut / onShortcut", () => {
  it("delivers only the matching action and unsubscribes", () => {
    const handler = vi.fn();
    const unsubscribe = onShortcut("run", handler);

    emitShortcut("run");
    emitShortcut("save");
    expect(handler).toHaveBeenCalledTimes(1);

    unsubscribe();
    emitShortcut("run");
    expect(handler).toHaveBeenCalledTimes(1);
  });
});
