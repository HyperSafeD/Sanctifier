import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import TerminalPage, { serializeLogs } from "./page";

const eventSources: MockEventSource[] = [];

class MockEventSource {
  onmessage: ((event: MessageEvent<string>) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  close = vi.fn();

  constructor(public readonly url: string) {
    eventSources.push(this);
  }

  emit(log: string) {
    this.onmessage?.({ data: JSON.stringify(log) } as MessageEvent<string>);
  }
}

describe("TerminalPage", () => {
  beforeEach(() => {
    eventSources.length = 0;
    vi.stubGlobal("EventSource", MockEventSource);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("serializes logs as newline-delimited text", () => {
    expect(serializeLogs(["first log", "second log"])).toBe("first log\nsecond log");
  });

  it("disables export actions until logs are available", () => {
    render(<TerminalPage />);

    expect(screen.getByRole("button", { name: /download \.log/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /^copy$/i })).toBeDisabled();
  });

  it("copies the current terminal logs to the clipboard", async () => {
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);

    render(<TerminalPage />);
    await user.click(screen.getByRole("button", { name: /start new analysis/i }));

    act(() => {
      eventSources[0].emit("scanning contract");
      eventSources[0].emit("analysis complete");
    });

    const copyButton = screen.getByRole("button", { name: /^copy$/i });
    await waitFor(() => expect(copyButton).toBeEnabled());
    await user.click(copyButton);

    expect(writeText).toHaveBeenCalledWith("scanning contract\nanalysis complete");
    expect(screen.getByRole("status")).toHaveTextContent("Logs copied to clipboard.");
  });
});
