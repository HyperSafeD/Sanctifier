import { describe, it, expect } from "vitest";
import { useRef, useState } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  DashboardProvider,
  dashboardReducer,
  initialDashboardState,
  useDashboardActions,
  useDashboardState,
  type DashboardState,
} from "./DashboardProvider";

describe("dashboardReducer", () => {
  it("ignores an unknown action", () => {
    const next = dashboardReducer(
      initialDashboardState,
      { type: "not-a-real-action" } as never,
    );

    expect(next).toBe(initialDashboardState);
  });

  it("does not mutate the previous state", () => {
    const before = { ...initialDashboardState };

    dashboardReducer(initialDashboardState, { type: "setActiveTab", activeTab: "diff" });

    expect(initialDashboardState).toEqual(before);
  });

  it("merges cached sources instead of replacing them", () => {
    const withOne = dashboardReducer(initialDashboardState, {
      type: "cacheSources",
      sources: { "a.rs": "fn a() {}" },
    });
    const withTwo = dashboardReducer(withOne, {
      type: "cacheSources",
      sources: { "b.rs": "fn b() {}" },
    });

    expect(withTwo.sourceCache).toEqual({ "a.rs": "fn a() {}", "b.rs": "fn b() {}" });
  });

  it("overwrites an existing cache entry on re-upload", () => {
    const first = dashboardReducer(initialDashboardState, {
      type: "cacheSources",
      sources: { "a.rs": "old" },
    });
    const second = dashboardReducer(first, {
      type: "cacheSources",
      sources: { "a.rs": "new" },
    });

    expect(second.sourceCache).toEqual({ "a.rs": "new" });
  });

  it("sets both the code filter and its validation message in one commit", () => {
    const next = dashboardReducer(initialDashboardState, {
      type: "setCodeFilter",
      codeFilterInput: "NOPE",
      codeFilterError: "Unknown finding code",
    });

    expect(next.codeFilterInput).toBe("NOPE");
    expect(next.codeFilterError).toBe("Unknown finding code");
  });

  it("clears the error and status when a parse starts", () => {
    const dirty: DashboardState = {
      ...initialDashboardState,
      error: "Invalid JSON",
      uploadStatus: "Analyzing…",
    };

    const next = dashboardReducer(dirty, { type: "parseStarted" });

    expect(next.error).toBeNull();
    expect(next.uploadStatus).toBeNull();
  });

  it("records a parse failure and drops any stale status", () => {
    const busy: DashboardState = { ...initialDashboardState, uploadStatus: "Analyzing…" };

    const next = dashboardReducer(busy, { type: "parseFailed", error: "Invalid JSON" });

    expect(next.error).toBe("Invalid JSON");
    expect(next.uploadStatus).toBeNull();
  });

  it("raises the in-flight flag and clears the error when an upload starts", () => {
    const failed: DashboardState = { ...initialDashboardState, error: "previous failure" };

    const next = dashboardReducer(failed, {
      type: "uploadStarted",
      batchProgress: { "a.rs": "analyzing" },
      uploadStatus: "Analyzing a.rs…",
    });

    expect(next.isUploadingContract).toBe(true);
    expect(next.error).toBeNull();
    expect(next.batchProgress).toEqual({ "a.rs": "analyzing" });
    expect(next.uploadStatus).toBe("Analyzing a.rs…");
  });

  it("replaces batch progress on a new upload rather than merging it", () => {
    const stale = dashboardReducer(initialDashboardState, {
      type: "uploadStarted",
      batchProgress: { "old.rs": "done" },
      uploadStatus: "…",
    });

    const next = dashboardReducer(stale, {
      type: "uploadStarted",
      batchProgress: { "new.rs": "analyzing" },
      uploadStatus: "…",
    });

    expect(next.batchProgress).toEqual({ "new.rs": "analyzing" });
  });

  it("updates one file's progress without disturbing its siblings", () => {
    const started = dashboardReducer(initialDashboardState, {
      type: "uploadStarted",
      batchProgress: { "a.rs": "pending", "b.rs": "pending" },
      uploadStatus: "Analyzing 2 files…",
    });

    const next = dashboardReducer(started, {
      type: "fileProgress",
      fileName: "b.rs",
      progress: "done",
    });

    expect(next.batchProgress).toEqual({ "a.rs": "pending", "b.rs": "done" });
  });

  it("lowers the in-flight flag on success", () => {
    const busy = dashboardReducer(initialDashboardState, {
      type: "uploadStarted",
      batchProgress: {},
      uploadStatus: "Analyzing…",
    });

    const next = dashboardReducer(busy, {
      type: "uploadSucceeded",
      uploadStatus: "Analysis report ready for a.rs.",
    });

    expect(next.isUploadingContract).toBe(false);
    expect(next.error).toBeNull();
    expect(next.uploadStatus).toBe("Analysis report ready for a.rs.");
  });

  it("lowers the in-flight flag and clears the status on failure", () => {
    const busy = dashboardReducer(initialDashboardState, {
      type: "uploadStarted",
      batchProgress: {},
      uploadStatus: "Analyzing…",
    });

    const next = dashboardReducer(busy, {
      type: "uploadFailed",
      error: "Contract analysis failed",
    });

    expect(next.isUploadingContract).toBe(false);
    expect(next.uploadStatus).toBeNull();
    expect(next.error).toBe("Contract analysis failed");
  });

  it("clears rejected files", () => {
    const rejected = dashboardReducer(initialDashboardState, {
      type: "rejectFiles",
      rejectedFiles: [{ name: "bad.txt", reason: "Unsupported extension" }],
    });
    expect(rejected.rejectedFiles).toHaveLength(1);

    expect(dashboardReducer(rejected, { type: "clearRejectedFiles" }).rejectedFiles).toEqual([]);
  });
});

// ── Provider wiring ───────────────────────────────────────────────────────────

/** Renders the context through the DOM so tests assert on the public surface. */
function Probe() {
  const state = useDashboardState();
  const actions = useDashboardActions();
  // Captured on the first render, then compared from an event handler on a
  // later render to prove `actions` keeps its identity across state changes.
  const firstActions = useRef(actions);
  const [actionsStable, setActionsStable] = useState("unchecked");

  return (
    <div>
      <span data-testid="tab">{state.activeTab}</span>
      <span data-testid="severity">{state.severityFilter}</span>
      <span data-testid="uploading">{String(state.isUploadingContract)}</span>
      <span data-testid="error">{state.error ?? "none"}</span>
      <span data-testid="status">{state.uploadStatus ?? "none"}</span>
      <span data-testid="actions-stable">{actionsStable}</span>
      <button onClick={() => actions.setActiveTab("diff")}>Go to diff</button>
      <button onClick={() => setActionsStable(String(actions === firstActions.current))}>
        Check stability
      </button>
      <button onClick={() => actions.uploadStarted({ "a.rs": "analyzing" }, "Analyzing a.rs…")}>
        Start upload
      </button>
      <button onClick={() => actions.uploadFailed("Contract analysis failed")}>
        Fail upload
      </button>
    </div>
  );
}

function renderProbe(initialState?: DashboardState) {
  return render(
    <DashboardProvider {...(initialState ? { initialState } : {})}>
      <Probe />
    </DashboardProvider>,
  );
}

describe("DashboardProvider", () => {
  it("seeds from the default state", () => {
    renderProbe();

    expect(screen.getByTestId("tab")).toHaveTextContent("findings");
    expect(screen.getByTestId("severity")).toHaveTextContent("all");
    expect(screen.getByTestId("uploading")).toHaveTextContent("false");
  });

  it("accepts a seeded initial state", () => {
    renderProbe({
      ...initialDashboardState,
      activeTab: "callgraph",
      severityFilter: "high",
    });

    expect(screen.getByTestId("tab")).toHaveTextContent("callgraph");
    expect(screen.getByTestId("severity")).toHaveTextContent("high");
  });

  it("applies an action dispatched from a consumer", async () => {
    const user = userEvent.setup();
    renderProbe();

    await user.click(screen.getByRole("button", { name: "Go to diff" }));

    expect(screen.getByTestId("tab")).toHaveTextContent("diff");
  });

  it("keeps the actions object referentially stable across state changes", async () => {
    const user = userEvent.setup();
    renderProbe();

    // Change state first, so the comparison runs against a *later* render's
    // `actions` rather than the one captured on mount.
    await user.click(screen.getByRole("button", { name: "Go to diff" }));
    await user.click(screen.getByRole("button", { name: "Check stability" }));

    // Stability is what lets `actions` sit in useCallback dependency arrays
    // without re-creating every handler on each keystroke.
    expect(screen.getByTestId("tab")).toHaveTextContent("diff");
    expect(screen.getByTestId("actions-stable")).toHaveTextContent("true");
  });

  it("threads a multi-field upload transition through in one render", async () => {
    const user = userEvent.setup();
    renderProbe();

    await user.click(screen.getByRole("button", { name: "Start upload" }));
    expect(screen.getByTestId("uploading")).toHaveTextContent("true");
    expect(screen.getByTestId("status")).toHaveTextContent("Analyzing a.rs…");

    await user.click(screen.getByRole("button", { name: "Fail upload" }));
    expect(screen.getByTestId("uploading")).toHaveTextContent("false");
    expect(screen.getByTestId("error")).toHaveTextContent("Contract analysis failed");
    expect(screen.getByTestId("status")).toHaveTextContent("none");
  });

  it("throws a helpful error when the state hook is used outside the provider", () => {
    function Orphan() {
      useDashboardState();
      return null;
    }

    expect(() => render(<Orphan />)).toThrow(/must be used within DashboardProvider/);
  });

  it("throws a helpful error when the actions hook is used outside the provider", () => {
    function Orphan() {
      useDashboardActions();
      return null;
    }

    expect(() => render(<Orphan />)).toThrow(/must be used within DashboardProvider/);
  });
});
