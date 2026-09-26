import { describe, it, expect, vi, afterEach } from "vitest";
import { act, render, screen } from "@testing-library/react";
import { AUTO_DISMISS_MS, ToastProvider, useOptionalToast, useToast } from "./ToastProvider";

function Trigger() {
  const toast = useToast();
  return <button onClick={() => toast.success("Saved!")}>go</button>;
}

afterEach(() => vi.useRealTimers());

describe("ToastProvider", () => {
  it("auto-dismisses after 5 seconds", () => {
    vi.useFakeTimers();
    render(
      <ToastProvider>
        <Trigger />
      </ToastProvider>,
    );
    act(() => screen.getByText("go").click());
    expect(screen.getByText("Saved!")).toBeInTheDocument();

    expect(AUTO_DISMISS_MS).toBe(5000);
    act(() => vi.advanceTimersByTime(4900));
    expect(screen.getByText("Saved!")).toBeInTheDocument();
    act(() => vi.advanceTimersByTime(200));
    expect(screen.queryByText("Saved!")).toBeNull();
  });

  it("useOptionalToast is a safe no-op without a provider", () => {
    function Probe() {
      const toast = useOptionalToast();
      toast.error("nobody is listening");
      return <p>rendered</p>;
    }
    render(<Probe />);
    expect(screen.getByText("rendered")).toBeInTheDocument();
  });
});
