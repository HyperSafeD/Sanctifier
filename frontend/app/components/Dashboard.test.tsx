import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Dashboard, type DashboardProps } from "./Dashboard";
import { createFinding, createFindingList } from "../../tests/fixtures";
import type { DashboardTab } from "../providers/DashboardProvider";

// ── Mock heavy sub-components ──────────────────────────────────────────────────

vi.mock("./SanctityScore", () => ({
  SanctityScore: ({ findings }: { findings: unknown[] }) => (
    <div data-testid="sanctity-score">score:{findings.length}</div>
  ),
}));

vi.mock("./TrendPanel", () => ({
  TrendPanel: ({
    records,
    onClear,
  }: {
    records: unknown[];
    onClear?: () => void;
  }) => (
    <div data-testid="trend-panel">
      trend:{records.length}
      {onClear && (
        <button data-testid="trend-clear" onClick={onClear}>
          Clear
        </button>
      )}
    </div>
  ),
}));

vi.mock("./FindingsList", () => ({
  FindingsList: ({
    findings,
    severityFilter,
    codeFilter,
  }: {
    findings: unknown[];
    severityFilter: string;
    codeFilter: string;
  }) => (
    <div data-testid="findings-list">
      <span data-testid="findings-count">{findings.length}</span>
      <span data-testid="severity-filter-val">{severityFilter}</span>
      <span data-testid="code-filter-val">{codeFilter}</span>
    </div>
  ),
}));

vi.mock("./SeverityFilter", () => ({
  SeverityFilter: ({
    selected,
    onChange,
  }: {
    selected: string;
    onChange: (v: string) => void;
  }) => (
    <div data-testid="severity-filter">
      <span data-testid="selected-severity">{selected}</span>
      <button data-testid="set-high" onClick={() => onChange("high")}>
        high
      </button>
    </div>
  ),
}));

vi.mock("./ErrorBoundary", () => ({
  ErrorBoundary: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

vi.mock("./ComparisonView", () => ({
  ComparisonView: ({
    baselineReport,
    currentReport,
    baselineName,
    currentName,
  }: {
    baselineReport: unknown;
    currentReport: unknown;
    baselineName: string;
    currentName: string;
  }) => (
    <div data-testid="comparison-view">
      <span data-testid="comparison-baseline">{baselineName}</span>
      <span data-testid="comparison-current">{currentName}</span>
      <span data-testid="comparison-has-baseline">
        {baselineReport ? "yes" : "no"}
      </span>
      <span data-testid="comparison-has-current">
        {currentReport ? "yes" : "no"}
      </span>
    </div>
  ),
}));

// ── Fixture helpers ────────────────────────────────────────────────────────────

function buildProps(overrides: Partial<DashboardProps> = {}): DashboardProps {
  return {
    findings: [],
    activeTab: "findings",
    onTabChange: vi.fn(),
    severityFilter: "all",
    onSeverityFilterChange: vi.fn(),
    codeFilter: "",
    codeFilterError: null,
    codeFilterInput: "",
    onCodeFilterChange: vi.fn(),
    baselineReport: null,
    currentReport: null,
    currentContractName: "my-contract",
    trendRecords: [],
    onClearHistory: vi.fn(),
    baselineJsonInput: "",
    onBaselineJsonInputChange: vi.fn(),
    callGraphPanel: <div data-testid="call-graph">CallGraph</div>,
    ...overrides,
  };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("Dashboard", () => {
  describe("Tab bar rendering", () => {
    it("renders all three tab buttons", () => {
      render(<Dashboard {...buildProps()} />);

      expect(screen.getByRole("tab", { name: "Findings" })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: "Call Graph" })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: "Diff" })).toBeInTheDocument();
    });

    it("marks the active tab with aria-selected=true", () => {
      render(<Dashboard {...buildProps({ activeTab: "callgraph" })} />);

      expect(
        screen.getByRole("tab", { name: "Call Graph" })
      ).toHaveAttribute("aria-selected", "true");
      expect(
        screen.getByRole("tab", { name: "Findings" })
      ).toHaveAttribute("aria-selected", "false");
      expect(
        screen.getByRole("tab", { name: "Diff" })
      ).toHaveAttribute("aria-selected", "false");
    });

    it("calls onTabChange with the correct tab when a tab is clicked", async () => {
      const onTabChange = vi.fn();
      const user = userEvent.setup();
      render(<Dashboard {...buildProps({ onTabChange })} />);

      await user.click(screen.getByRole("tab", { name: "Diff" }));
      expect(onTabChange).toHaveBeenCalledWith("diff");

      await user.click(screen.getByRole("tab", { name: "Call Graph" }));
      expect(onTabChange).toHaveBeenCalledWith("callgraph");
    });

    it("exposes a tablist with the expected aria-label", () => {
      render(<Dashboard {...buildProps()} />);
      expect(
        screen.getByRole("tablist", { name: "Analysis view tabs" })
      ).toBeInTheDocument();
    });
  });

  describe("Findings tab", () => {
    it("renders findings panel when activeTab is 'findings'", () => {
      render(<Dashboard {...buildProps({ activeTab: "findings" })} />);

      expect(screen.getByRole("tabpanel")).toBeInTheDocument();
      expect(screen.getByTestId("findings-list")).toBeInTheDocument();
    });

    it("passes the finding count to FindingsList", () => {
      const findings = createFindingList(5);
      render(<Dashboard {...buildProps({ findings, activeTab: "findings" })} />);

      expect(screen.getByTestId("findings-count")).toHaveTextContent("5");
    });

    it("passes severityFilter to FindingsList", () => {
      render(
        <Dashboard
          {...buildProps({ activeTab: "findings", severityFilter: "critical" })}
        />
      );
      expect(screen.getByTestId("severity-filter-val")).toHaveTextContent(
        "critical"
      );
    });

    it("passes empty codeFilter to FindingsList when there is a codeFilterError", () => {
      render(
        <Dashboard
          {...buildProps({
            activeTab: "findings",
            codeFilter: "S999",
            codeFilterError: "Unknown finding code",
          })}
        />
      );
      expect(screen.getByTestId("code-filter-val")).toHaveTextContent("");
    });

    it("passes the validated codeFilter when there is no error", () => {
      render(
        <Dashboard
          {...buildProps({
            activeTab: "findings",
            codeFilter: "S001",
            codeFilterError: null,
          })}
        />
      );
      expect(screen.getByTestId("code-filter-val")).toHaveTextContent("S001");
    });

    it("calls onSeverityFilterChange when severity changes", async () => {
      const onSeverityFilterChange = vi.fn();
      const user = userEvent.setup();
      render(
        <Dashboard
          {...buildProps({ activeTab: "findings", onSeverityFilterChange })}
        />
      );

      await user.click(screen.getByTestId("set-high"));
      expect(onSeverityFilterChange).toHaveBeenCalledWith("high");
    });

    it("calls onCodeFilterChange when the code filter input changes", () => {
      const onCodeFilterChange = vi.fn();
      render(
        <Dashboard
          {...buildProps({ activeTab: "findings", onCodeFilterChange })}
        />
      );

      const input = screen.getByPlaceholderText("S001");
      fireEvent.change(input, { target: { value: "S003" } });
      expect(onCodeFilterChange).toHaveBeenCalledWith("S003");
    });

    it("shows a code filter validation error when codeFilterError is set", () => {
      render(
        <Dashboard
          {...buildProps({
            activeTab: "findings",
            codeFilterError: "Unknown finding code",
          })}
        />
      );
      expect(screen.getByText("Unknown finding code")).toBeInTheDocument();
    });

    it("shows the helper text when there is no codeFilterError", () => {
      render(
        <Dashboard
          {...buildProps({ activeTab: "findings", codeFilterError: null })}
        />
      );
      expect(
        screen.getByText("Use exact finding codes like S001, S012, or S020.")
      ).toBeInTheDocument();
    });

    it("marks the code filter input aria-invalid when there is an error", () => {
      render(
        <Dashboard
          {...buildProps({
            activeTab: "findings",
            codeFilterError: "Invalid code",
          })}
        />
      );
      expect(screen.getByPlaceholderText("S001")).toHaveAttribute(
        "aria-invalid",
        "true"
      );
    });

    it("shows empty-state message when there are no findings", () => {
      render(
        <Dashboard {...buildProps({ findings: [], activeTab: "findings" })} />
      );
      expect(
        screen.getByText(/No findings to display/)
      ).toBeInTheDocument();
    });

    it("does not show empty-state message when there are findings", () => {
      render(
        <Dashboard
          {...buildProps({
            findings: [createFinding()],
            activeTab: "findings",
          })}
        />
      );
      expect(
        screen.queryByText(/No findings to display/)
      ).not.toBeInTheDocument();
    });
  });

  describe("Call graph tab", () => {
    it("renders the callGraphPanel slot when activeTab is 'callgraph'", () => {
      render(<Dashboard {...buildProps({ activeTab: "callgraph" })} />);

      expect(screen.getByTestId("call-graph")).toBeInTheDocument();
    });

    it("does not render the findings panel on the callgraph tab", () => {
      render(<Dashboard {...buildProps({ activeTab: "callgraph" })} />);

      expect(screen.queryByTestId("findings-list")).not.toBeInTheDocument();
    });
  });

  describe("Diff tab", () => {
    it("renders the diff tab panel when activeTab is 'diff'", () => {
      render(<Dashboard {...buildProps({ activeTab: "diff" })} />);

      expect(screen.getByTestId("comparison-view")).toBeInTheDocument();
    });

    it("shows baseline textarea with correct placeholder", () => {
      render(<Dashboard {...buildProps({ activeTab: "diff" })} />);

      expect(
        screen.getByPlaceholderText("Paste baseline JSON report here...")
      ).toBeInTheDocument();
    });

    it("passes baselineJsonInput as textarea value", () => {
      render(
        <Dashboard
          {...buildProps({ activeTab: "diff", baselineJsonInput: '{"test":1}' })}
        />
      );

      const textarea = screen.getByPlaceholderText(
        "Paste baseline JSON report here..."
      ) as HTMLTextAreaElement;
      expect(textarea.value).toBe('{"test":1}');
    });

    it("calls onBaselineJsonInputChange when the textarea changes", () => {
      const onBaselineJsonInputChange = vi.fn();
      render(
        <Dashboard
          {...buildProps({ activeTab: "diff", onBaselineJsonInputChange })}
        />
      );

      fireEvent.change(
        screen.getByPlaceholderText("Paste baseline JSON report here..."),
        { target: { value: "{}" } }
      );
      expect(onBaselineJsonInputChange).toHaveBeenCalledWith("{}");
    });

    it("passes currentContractName to ComparisonView", () => {
      render(
        <Dashboard
          {...buildProps({
            activeTab: "diff",
            currentContractName: "awesome-contract",
          })}
        />
      );
      expect(screen.getByTestId("comparison-current")).toHaveTextContent(
        "awesome-contract"
      );
    });

    it("shows 'no current report' message when currentReport is null", () => {
      render(
        <Dashboard
          {...buildProps({ activeTab: "diff", currentReport: null })}
        />
      );
      expect(
        screen.getByText("No current report loaded. Upload a contract first.")
      ).toBeInTheDocument();
    });

    it("shows 'current report loaded' message when currentReport is present", () => {
      const currentReport = { rule_violations: [], error_codes: [], summary: { total_findings: 0 } } as any;
      render(
        <Dashboard
          {...buildProps({ activeTab: "diff", currentReport })}
        />
      );
      expect(
        screen.getByText("Current report loaded from the active contract.")
      ).toBeInTheDocument();
    });

    it("passes baseline presence to ComparisonView", () => {
      const baseline = { rule_violations: [], error_codes: [], summary: { total_findings: 0 } } as any;
      render(
        <Dashboard
          {...buildProps({ activeTab: "diff", baselineReport: baseline })}
        />
      );
      expect(screen.getByTestId("comparison-has-baseline")).toHaveTextContent(
        "yes"
      );
    });
  });

  describe("Summary row (always visible)", () => {
    it("renders SanctityScore with the findings count", () => {
      const findings = createFindingList(3);
      render(<Dashboard {...buildProps({ findings })} />);
      expect(screen.getByTestId("sanctity-score")).toHaveTextContent("score:3");
    });

    it("renders TrendPanel with the record count", () => {
      const trendRecords = [
        { workspace: "ws", timestamp: 1, critical: 0, high: 0, medium: 0, low: 0, total: 0 },
      ];
      render(<Dashboard {...buildProps({ trendRecords })} />);
      expect(screen.getByTestId("trend-panel")).toHaveTextContent("trend:1");
    });

    it("calls onClearHistory when TrendPanel clear button is clicked", async () => {
      const onClearHistory = vi.fn();
      const user = userEvent.setup();
      const trendRecords = [
        { workspace: "ws", timestamp: 1, critical: 0, high: 0, medium: 0, low: 0, total: 0 },
      ];
      render(<Dashboard {...buildProps({ trendRecords, onClearHistory })} />);

      await user.click(screen.getByTestId("trend-clear"));
      expect(onClearHistory).toHaveBeenCalledOnce();
    });
  });

  describe("Tab panel visibility isolation", () => {
    const tabs: DashboardTab[] = ["findings", "callgraph", "diff"];

    it.each(tabs)(
      "only the '%s' tab panel is visible when that tab is active",
      (tab) => {
        render(<Dashboard {...buildProps({ activeTab: tab })} />);

        const panel = screen.getByRole("tabpanel");
        expect(panel).toBeInTheDocument();

        // Verify only one tabpanel is rendered at a time
        expect(screen.getAllByRole("tabpanel")).toHaveLength(1);
      }
    );
  });

  describe("Edge cases", () => {
    it("renders with a large findings list without crashing", () => {
      const findings = createFindingList(500);
      expect(() =>
        render(<Dashboard {...buildProps({ findings, activeTab: "findings" })} />)
      ).not.toThrow();
      expect(screen.getByTestId("findings-count")).toHaveTextContent("500");
    });

    it("renders with all severity types without crashing", () => {
      for (const severity of ["critical", "high", "medium", "low", "all"] as const) {
        const { unmount } = render(
          <Dashboard
            {...buildProps({ severityFilter: severity === "all" ? "all" : severity, activeTab: "findings" })}
          />
        );
        expect(screen.getByTestId("selected-severity")).toHaveTextContent(severity);
        unmount();
      }
    });

    it("renders without optional getSource and getFileName props", () => {
      const props = buildProps({ activeTab: "findings" });
      delete props.getSource;
      delete props.getFileName;
      expect(() => render(<Dashboard {...props} />)).not.toThrow();
    });

    it("renders without a callGraphPanel slot on the callgraph tab", () => {
      expect(() =>
        render(
          <Dashboard
            {...buildProps({ activeTab: "callgraph", callGraphPanel: undefined })}
          />
        )
      ).not.toThrow();
    });
  });
});
