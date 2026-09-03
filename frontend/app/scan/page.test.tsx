import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import ScanPage from "./page";
import { ScanProvider } from "./ScanContext";
import type { Finding, Severity } from "../types";
import { createFinding, createFindingList } from "../../tests/fixtures";

// Mock the dynamic import for CallGraph
vi.mock("../components/CallGraph", () => ({
  default: () => <div data-testid="mock-call-graph">Call Graph Component</div>,
  CallGraph: () => <div data-testid="mock-call-graph">Call Graph Component</div>,
}));

vi.mock("next/dynamic", () => ({
  default: () => () => <div data-testid="mock-call-graph">Call Graph Component</div>,
}));

// Mock the child components that are used in the scan page
vi.mock("../components/AnalysisTerminal", () => ({
  AnalysisTerminal: ({ logs, isAnalyzing }: { logs: string[]; isAnalyzing: boolean }) => (
    <div data-testid="mock-analysis-terminal">
      <div data-testid="terminal-logs">{logs.join(", ")}</div>
      <div data-testid="terminal-analyzing">{isAnalyzing ? "true" : "false"}</div>
    </div>
  ),
}));

vi.mock("../components/SanctityScore", () => ({
  SanctityScore: ({ findings }: { findings: Finding[] }) => (
    <div data-testid="mock-sanctity-score">
      <div data-testid="score-findings-count">{findings.length}</div>
    </div>
  ),
}));

vi.mock("../components/FindingsList", () => ({
  FindingsList: ({ findings, severityFilter }: { findings: Finding[]; severityFilter: Severity | "all" }) => (
    <div data-testid="mock-findings-list">
      <div data-testid="findings-count">{findings.length}</div>
      <div data-testid="severity-filter">{severityFilter}</div>
    </div>
  ),
}));

vi.mock("../components/ZkFindingsPanel", () => ({
  ZkFindingsPanel: ({ findings }: { findings: Finding[] }) => (
    <div data-testid="mock-zk-findings-panel">
      <div data-testid="zk-findings-count">{findings.length}</div>
    </div>
  ),
}));

vi.mock("../components/SeverityFilter", () => ({
  SeverityFilter: ({ selected, onChange }: { selected: Severity | "all"; onChange: (filter: Severity | "all") => void }) => (
    <div data-testid="mock-severity-filter">
      <button data-testid="filter-button" onClick={() => onChange("critical")}>
        Set to critical
      </button>
      <div data-testid="current-filter">{selected}</div>
    </div>
  ),
}));

vi.mock("../components/ErrorBoundary", () => ({
  ErrorBoundary: ({ children, compact }: { children: React.ReactNode; compact?: boolean }) => (
    <div data-testid={`mock-error-boundary-${compact ? "compact" : "normal"}`}>
      {children}
    </div>
  ),
}));

vi.mock("../components/CallGraphSkeleton", () => ({
  CallGraphSkeleton: () => <div data-testid="mock-call-graph-skeleton">Loading Call Graph...</div>,
}));

vi.mock("../lib/scan-progress", () => ({
  nextScanProgressPhase: (phaseIndex: number) => `Phase ${phaseIndex}: Mock progress`,
}));

vi.mock("../lib/settings", () => ({
  getSettingsHeaders: () => ({ "X-Test-Header": "test-value" }),
}));

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

// Mock File constructor
class MockFile extends Blob {
  name: string;
  lastModified: number;

  constructor(
    fileBits: BlobPart[],
    fileName: string,
    options?: FilePropertyBag
  ) {
    super(fileBits, options);
    this.name = fileName;
    this.lastModified = Date.now();
  }
}

global.File = MockFile as any;

// Mock clipboard API
Object.defineProperty(navigator, "clipboard", {
  value: {
    writeText: vi.fn(),
  },
  writable: true,
  configurable: true,
});

// Mock window.alert
global.alert = vi.fn();

// Create test utilities
const createMockFile = (name = "contract.rs", size = 1024) => {
  const content = "pub fn test() {}";
  const blob = new Blob([content], { type: "text/plain" });
  return new File([blob], name, { type: "text/plain" });
};

const createMockResponse = (data: any, ok = true) => ({
  ok,
  json: () => Promise.resolve(data),
  text: () => Promise.resolve(JSON.stringify(data)),
});

const renderScanPage = () => {
  return render(
    <ScanProvider>
      <ScanPage />
    </ScanProvider>
  );
};

describe("ScanPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockFetch.mockReset();
    vi.useFakeTimers({ shouldAdvanceTime: true });
    Object.defineProperty(navigator, "clipboard", {
      value: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
      writable: true,
      configurable: true,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Initial render", () => {
    it("renders the page header correctly", () => {
      renderScanPage();

      expect(screen.getByText("Security Scanner")).toBeInTheDocument();
      expect(
        screen.getByText(
          "Upload your Soroban contract source file (.rs) for an instant deep-dive security audit."
        )
      ).toBeInTheDocument();
    });

    it("renders file upload area", () => {
      renderScanPage();

      const uploadLabel = screen.getByLabelText("Choose a Soroban contract source file to scan");
      expect(uploadLabel).toBeInTheDocument();
      expect(screen.getByText("Choose a Rust contract")).toBeInTheDocument();
      expect(
        screen.getByText("Click to browse or drag and drop your .rs file")
      ).toBeInTheDocument();
    });

    it("renders run audit button disabled initially", () => {
      renderScanPage();

      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });
      expect(runButton).toBeInTheDocument();
      expect(runButton).toBeDisabled();
    });

    it("does not show terminal section initially", () => {
      renderScanPage();

      expect(screen.queryByText("Live Analysis Stream")).not.toBeInTheDocument();
      expect(screen.queryByTestId("mock-analysis-terminal")).not.toBeInTheDocument();
    });

    it("does not show error section initially", () => {
      renderScanPage();

      expect(screen.queryByText("Analysis Failed")).not.toBeInTheDocument();
    });

    it("does not show results section initially", () => {
      renderScanPage();

      expect(screen.queryByText("Analysis Summary")).not.toBeInTheDocument();
      expect(screen.queryByText("Security Findings")).not.toBeInTheDocument();
    });
  });

  describe("File upload functionality", () => {
    it("handles file selection via input", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const file = createMockFile("test-contract.rs");
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");

      await user.upload(fileInput, file);

      expect(screen.getByText("test-contract.rs")).toBeInTheDocument();
    });

    it("enables run audit button when file is selected", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      expect(runButton).toBeDisabled();

      await user.upload(fileInput, file);

      expect(runButton).toBeEnabled();
    });

    it("resets scan state when new file is selected", async () => {
      const user = userEvent.setup();
      renderScanPage();

      // First file
      const file1 = createMockFile("first.rs");
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      await user.upload(fileInput, file1);

      expect(screen.getByText("first.rs")).toBeInTheDocument();

      // Second file - should reset
      const file2 = createMockFile("second.rs");
      await user.upload(fileInput, file2);

      expect(screen.getByText("second.rs")).toBeInTheDocument();
      expect(screen.queryByText("first.rs")).not.toBeInTheDocument();
    });

    it("disables file input during analysis", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      // Select file and start analysis
      await user.upload(fileInput, file);
      await user.click(runButton);

      // Check that file input is disabled during analysis
      expect(fileInput).toBeDisabled();
    });

    it("handles file selection with different file extensions", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");

      // Test with .rs extension
      const rustFile = createMockFile("contract.rs");
      await user.upload(fileInput, rustFile);
      expect(screen.getByText("contract.rs")).toBeInTheDocument();

      // Test with different extension (should still work as browser handles accept attribute)
      const otherFile = createMockFile("contract.txt");
      await user.upload(fileInput, otherFile, { applyAccept: false });
      expect(screen.getByText("contract.txt")).toBeInTheDocument();
    });

    it("maintains file selection after component re-render", async () => {
      const user = userEvent.setup();
      const { rerender } = renderScanPage();

      const file = createMockFile("persistent.rs");
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");

      await user.upload(fileInput, file);
      expect(screen.getByText("persistent.rs")).toBeInTheDocument();

      // Simulate re-render
      rerender(
        <ScanProvider>
          <ScanPage />
        </ScanProvider>
      );

      // File should still be selected
      expect(screen.getByText("persistent.rs")).toBeInTheDocument();
    });

    it("shows visual feedback when file is selected", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const uploadArea = fileInput.closest('label');

      // Check initial state
      expect(uploadArea).toHaveClass("border-zinc-200");

      // Upload file
      const file = createMockFile();
      await user.upload(fileInput, file);

      // Check visual feedback for selected file
      expect(uploadArea).toHaveClass("border-emerald-500/50");
      expect(uploadArea).toHaveClass("bg-emerald-500/5");
    });

    it("handles empty file selection (no file chosen)", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      
      // Simulate clicking but not selecting a file
      await user.click(fileInput);
      
      // Should remain in initial state
      expect(screen.getByText("Choose a Rust contract")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /Run Security Audit/i })).toBeDisabled();
    });

    it("validates file input accept attribute", () => {
      renderScanPage();

      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      
      // Check that input has correct accept attribute for .rs files
      expect(fileInput).toHaveAttribute("accept", ".rs");
    });
  });

  describe("Analysis execution and API calls", () => {
    it("executes analysis when run audit button is clicked", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [createFinding(), createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      // Verify API call was made
      expect(mockFetch).toHaveBeenCalledTimes(1);
      expect(mockFetch).toHaveBeenCalledWith(
        "/api/analyze",
        expect.objectContaining({
          method: "POST",
          headers: { "X-Test-Header": "test-value" },
        })
      );

      // Verify FormData includes the file
      const formData = mockFetch.mock.calls[0][1].body;
      expect(formData).toBeInstanceOf(FormData);
      
      // Check button shows loading state
      expect(runButton).toHaveTextContent("Running Audit...");
      expect(runButton).toBeDisabled();
    });

    it("shows progress logs during analysis", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [createFinding()];
      mockFetch.mockReturnValueOnce(new Promise(resolve => setTimeout(() => resolve(createMockResponse(mockFindings)), 5000)));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      // Advance timers to trigger progress logs
      act(() => {
        vi.advanceTimersByTime(1500); // First log
        vi.advanceTimersByTime(1500); // Second log
      });

      // Check that terminal section appears
      expect(screen.getByText("Live Analysis Stream")).toBeInTheDocument();
      expect(screen.getByTestId("mock-analysis-terminal")).toBeInTheDocument();

      // Check that logs are being added
      const terminalLogs = screen.getByTestId("terminal-logs");
      expect(terminalLogs.textContent).toContain("Phase 0: Mock progress");
      expect(terminalLogs.textContent).toContain("Phase 1: Mock progress");
    });

    it("clears progress timer when analysis completes", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [createFinding()];
      mockFetch.mockReturnValueOnce(new Promise(resolve => setTimeout(() => resolve(createMockResponse(mockFindings)), 5000)));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      // Start timer
      act(() => {
        vi.advanceTimersByTime(1500);
      });

      act(() => {
        vi.advanceTimersByTime(5000);
      });

      // Wait for analysis to complete
      await waitFor(() => {
        expect(screen.queryByText("Running Audit...")).not.toBeInTheDocument();
      });

      // Advance timers further - should not trigger more logs after completion
      act(() => {
        vi.advanceTimersByTime(3000);
      });

      // Only the initial log should be present
      const terminalLogs = screen.getByTestId("terminal-logs");
      expect(terminalLogs.textContent).toContain("Phase 0: Mock progress");
      expect(terminalLogs.textContent).not.toContain("Phase 2: Mock progress");
    });

    it("handles successful analysis response", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [
        createFinding({ title: "Critical Vulnerability", severity: "critical" }),
        createFinding({ title: "Medium Issue", severity: "medium" }),
      ];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      // Wait for analysis to complete
      await waitFor(() => {
        expect(runButton).toBeEnabled();
        expect(runButton).toHaveTextContent("Run Security Audit");
      });

      // Check results section appears
      expect(screen.getByText("Analysis Summary")).toBeInTheDocument();
      expect(screen.getByText("Security Findings")).toBeInTheDocument();

      // Check findings are passed to child components
      expect(screen.getByTestId("score-findings-count")).toHaveTextContent("2");
      expect(screen.getByTestId("findings-count")).toHaveTextContent("2");
      expect(screen.getByTestId("zk-findings-count")).toHaveTextContent("2");
    });

    it("updates logs with success message on completion", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [createFinding(), createFinding(), createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile("test.rs");
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(runButton).toBeEnabled();
      });

      // Check success logs
      const terminalLogs = screen.getByTestId("terminal-logs");
      expect(terminalLogs.textContent).toContain("Starting analysis for test.rs");
      expect(terminalLogs.textContent).toContain("Uploading contract to analysis engine");
      expect(terminalLogs.textContent).toContain("Analysis complete. Found 3 potential issues");
      expect(terminalLogs.textContent).toContain("SUCCESS: Security report generated");
    });

    it("does not execute analysis without selected file", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      // Button should be disabled without file
      expect(runButton).toBeDisabled();

      // Try to click disabled button
      await user.click(runButton);

      // No API call should be made
      expect(mockFetch).not.toHaveBeenCalled();
    });

    it("prevents multiple concurrent analysis runs", async () => {
      const user = userEvent.setup();
      renderScanPage();

      // Mock a slow response
      const mockFindings = [createFinding()];
      mockFetch.mockImplementationOnce(
        () => new Promise((resolve) => setTimeout(() => resolve(createMockResponse(mockFindings)), 1000))
      );

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      
      // Start first analysis
      await user.click(runButton);
      expect(runButton).toBeDisabled();
      expect(runButton).toHaveTextContent("Running Audit...");

      // Try to click again while analysis is running
      await user.click(runButton);

      // Should still be only one API call
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    it("clears previous findings and logs when starting new analysis", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings1 = [createFinding({ title: "First Finding" })];
      const mockFindings2 = [createFinding({ title: "Second Finding" })];
      
      mockFetch
        .mockResolvedValueOnce(createMockResponse(mockFindings1))
        .mockReturnValueOnce(new Promise(resolve => setTimeout(() => resolve(createMockResponse(mockFindings2)), 5000)));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      // First analysis
      await user.upload(fileInput, file);
      await user.click(runButton);
      await waitFor(() => {
        expect(runButton).toBeEnabled();
      });

      // Verify first results
      expect(screen.getByTestId("findings-count")).toHaveTextContent("1");

      // Start second analysis
      await user.click(runButton);

      // During second analysis, previous findings should be cleared
      expect(screen.queryByTestId("findings-count")).not.toBeInTheDocument();
      
      act(() => {
        vi.advanceTimersByTime(5000);
      });

      await waitFor(() => {
        expect(runButton).toBeEnabled();
      });

      // Should show second results
      expect(screen.getByTestId("findings-count")).toHaveTextContent("1");
    });
  });

  describe("Error states and error handling", () => {
    it("handles API error response", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const errorMessage = "Server error: Internal server error";
      mockFetch.mockResolvedValueOnce(
        createMockResponse({ error: errorMessage }, false)
      );

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Failed")).toBeInTheDocument();
        expect(screen.getByText(errorMessage)).toBeInTheDocument();
      });

      // Check error logs
      const terminalLogs = screen.getByTestId("terminal-logs");
      expect(terminalLogs.textContent).toContain(`ERROR: ${errorMessage}`);
    });

    it("handles network failure", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const errorMessage = "Network request failed";
      mockFetch.mockRejectedValueOnce(new Error(errorMessage));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Failed")).toBeInTheDocument();
        expect(screen.getByText(errorMessage)).toBeInTheDocument();
      });

      // Check that button re-enables after error
      expect(runButton).toBeEnabled();
      expect(runButton).toHaveTextContent("Run Security Audit");
    });

    it("shows error section with retry button", async () => {
      const user = userEvent.setup();
      renderScanPage();

      mockFetch.mockRejectedValueOnce(new Error("Test error"));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Failed")).toBeInTheDocument();
      });

      // Check retry button
      const retryButton = screen.getByRole("button", { name: /Try Again/i });
      expect(retryButton).toBeInTheDocument();

      // Mock successful response for retry
      const mockFindings = [createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      // Click retry button
      await user.click(retryButton);

      // Check that error section disappears and analysis runs again
      await waitFor(() => {
        expect(screen.queryByText("Analysis Failed")).not.toBeInTheDocument();
      });

      // Should make new API call
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });

    it("clears previous error when starting new analysis", async () => {
      const user = userEvent.setup();
      renderScanPage();

      // First attempt fails
      mockFetch.mockRejectedValueOnce(new Error("First error"));
      
      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Failed")).toBeInTheDocument();
      });

      // Mock successful response for second attempt
      const mockFindings = [createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      // Start new analysis (not using retry button, but regular button)
      await user.click(runButton);

      // Error section should disappear immediately when starting new analysis
      expect(screen.queryByText("Analysis Failed")).not.toBeInTheDocument();

      await waitFor(() => {
        expect(runButton).toBeEnabled();
      });

      // Should show results instead of error
      expect(screen.getByText("Analysis Summary")).toBeInTheDocument();
    });

    it("handles invalid JSON response", async () => {
      const user = userEvent.setup();
      renderScanPage();

      // Mock response that returns invalid JSON
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.reject(new Error("Invalid JSON")),
        text: () => Promise.resolve("Invalid JSON response"),
      });

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Failed")).toBeInTheDocument();
        expect(screen.getByText("Analysis failed")).toBeInTheDocument();
      });
    });

    it("handles empty error response from API", async () => {
      const user = userEvent.setup();
      renderScanPage();

      // Mock response with empty error field
      mockFetch.mockResolvedValueOnce(
        createMockResponse({}, false)
      );

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Failed")).toBeInTheDocument();
        expect(screen.getByText("Analysis failed")).toBeInTheDocument();
      });
    });

    it("maintains logs when error occurs", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const errorMessage = "Test error message";
      mockFetch.mockRejectedValueOnce(new Error(errorMessage));

      const file = createMockFile("error-test.rs");
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      // Advance timer to generate some progress logs before error
      act(() => {
        vi.advanceTimersByTime(1500);
      });

      await waitFor(() => {
        expect(screen.getByText("Analysis Failed")).toBeInTheDocument();
      });

      // Check that logs are preserved
      const terminalLogs = screen.getByTestId("terminal-logs");
      expect(terminalLogs.textContent).toContain("Starting analysis for error-test.rs");
      expect(terminalLogs.textContent).toContain("Uploading contract to analysis engine");
      expect(terminalLogs.textContent).toContain("Phase 0: Mock progress");
      expect(terminalLogs.textContent).toContain(`ERROR: ${errorMessage}`);
    });

    it("handles error during progress logging", async () => {
      const user = userEvent.setup();
      renderScanPage();

      // Mock a response that succeeds but has timer cleanup issues
      const mockFindings = [createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      // Start timer
      act(() => {
        vi.advanceTimersByTime(1500);
      });

      // Complete analysis
      await waitFor(() => {
        expect(runButton).toBeEnabled();
      });

      // Advance timers after completion - should not crash
      act(() => {
        vi.advanceTimersByTime(3000);
      });

      // Component should still be functional
      expect(screen.getByText("Analysis Summary")).toBeInTheDocument();
    });

    it("handles error with special characters in message", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const errorMessage = "Error with special chars: <>&\"'";
      mockFetch.mockRejectedValueOnce(new Error(errorMessage));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Failed")).toBeInTheDocument();
        // Error message should be safely displayed
        expect(screen.getByText(errorMessage)).toBeInTheDocument();
      });
    });
  });

  describe("Scan results display and filtering", () => {
    it("displays results section after successful analysis", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Summary")).toBeInTheDocument();
        expect(screen.getByText("Security Findings")).toBeInTheDocument();
        expect(screen.getByText("System Integrity Map")).toBeInTheDocument();
      });

      // Check all result sections are present
      expect(screen.getByTestId("mock-sanctity-score")).toBeInTheDocument();
      expect(screen.getByTestId("mock-findings-list")).toBeInTheDocument();
      expect(screen.getByTestId("mock-zk-findings-panel")).toBeInTheDocument();
      expect(screen.getByTestId("mock-call-graph")).toBeInTheDocument();
    });

    it("shows correct findings count in results", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = createFindingList(5);
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByTestId("score-findings-count")).toHaveTextContent("5");
        expect(screen.getByTestId("findings-count")).toHaveTextContent("5");
        expect(screen.getByTestId("zk-findings-count")).toHaveTextContent("5");
      });
    });

    it("includes dashboard link in results", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        const dashboardLink = screen.getByRole("link", { name: /Open Full Dashboard/i });
        expect(dashboardLink).toBeInTheDocument();
        expect(dashboardLink).toHaveAttribute("href", "/dashboard");
      });
    });

    it("handles share report functionality", async () => {
      const user = userEvent.setup();
      vi.spyOn(navigator.clipboard, "writeText");
      renderScanPage();

      const mockFindings = [createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Summary")).toBeInTheDocument();
      });

      // Click share button
      const shareButton = screen.getByRole("button", { name: /Share Report/i });
      await user.click(shareButton);

      // Check clipboard was called
      expect(navigator.clipboard.writeText).toHaveBeenCalledTimes(1);
      
      // Check alert was shown
      expect(global.alert).toHaveBeenCalledTimes(1);
      expect(global.alert).toHaveBeenCalledWith(
        expect.stringContaining("Shareable link copied to clipboard")
      );
    });

    it("applies severity filtering to findings", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [
        createFinding({ severity: "critical", title: "Critical Finding" }),
        createFinding({ severity: "high", title: "High Finding" }),
        createFinding({ severity: "medium", title: "Medium Finding" }),
      ];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByTestId("findings-count")).toHaveTextContent("3");
      });

      // Check initial filter state
      expect(screen.getByTestId("current-filter")).toHaveTextContent("all");

      // Simulate filter change via mock component
      const filterButton = screen.getByTestId("filter-button");
      await user.click(filterButton);

      // Check filter was updated
      expect(screen.getByTestId("current-filter")).toHaveTextContent("critical");
    });

    it("handles empty findings results", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings: Finding[] = [];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Summary")).toBeInTheDocument();
      });

      // Check empty state in child components
      expect(screen.getByTestId("score-findings-count")).toHaveTextContent("0");
      expect(screen.getByTestId("findings-count")).toHaveTextContent("0");
      expect(screen.getByTestId("zk-findings-count")).toHaveTextContent("0");
    });

    it("shows call graph section with mock data", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("System Integrity Map")).toBeInTheDocument();
      });

      // Check call graph is rendered
      expect(screen.getByTestId("mock-call-graph")).toBeInTheDocument();
      expect(screen.getByTestId("mock-call-graph")).toHaveTextContent("Call Graph Component");
    });

    it("updates results when filter changes via context", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [createFinding(), createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByTestId("current-filter")).toHaveTextContent("all");
      });

      // Change filter through the mock component
      const filterButton = screen.getByTestId("filter-button");
      await user.click(filterButton);

      // Verify filter was updated
      expect(screen.getByTestId("current-filter")).toHaveTextContent("critical");
    });

    it("hides results section during new analysis", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings1 = [createFinding()];
      const mockFindings2 = [createFinding(), createFinding()];
      
      mockFetch
        .mockResolvedValueOnce(createMockResponse(mockFindings1))
        .mockReturnValueOnce(new Promise(resolve => setTimeout(() => resolve(createMockResponse(mockFindings2)), 5000)));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      // First analysis
      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Summary")).toBeInTheDocument();
      });

      // Start second analysis
      await user.click(runButton);

      // Results should disappear during analysis
      expect(screen.queryByText("Analysis Summary")).not.toBeInTheDocument();
      expect(screen.queryByText("Security Findings")).not.toBeInTheDocument();

      act(() => {
        vi.advanceTimersByTime(5000);
      });

      // Wait for second analysis to complete
      await waitFor(() => {
        expect(screen.getByText("Analysis Summary")).toBeInTheDocument();
      });

      // New results should appear
      expect(screen.getByTestId("findings-count")).toHaveTextContent("2");
    });

    it("preserves results when selecting new file without analysis", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file1 = createMockFile("first.rs");
      const file2 = createMockFile("second.rs");
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      // Analyze first file
      await user.upload(fileInput, file1);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Summary")).toBeInTheDocument();
      });

      // Select new file (should clear results)
      await user.upload(fileInput, file2);

      // Results should be cleared when new file is selected
      expect(screen.queryByText("Analysis Summary")).not.toBeInTheDocument();
      expect(runButton).toBeEnabled(); // Should still be enabled with new file
    });

    it("shows appropriate visual styling for results sections", async () => {
      const user = userEvent.setup();
      renderScanPage();

      const mockFindings = [createFinding()];
      mockFetch.mockResolvedValueOnce(createMockResponse(mockFindings));

      const file = createMockFile();
      const fileInput = screen.getByLabelText("Choose a Soroban contract source file to scan");
      const runButton = screen.getByRole("button", { name: /Run Security Audit/i });

      await user.upload(fileInput, file);
      await user.click(runButton);

      await waitFor(() => {
        expect(screen.getByText("Analysis Summary")).toBeInTheDocument();
      });

      // Check that results sections have proper animations
      const resultsSection = screen.getByText("Analysis Summary").closest('section');
      expect(resultsSection).toHaveClass("animate-in");
      expect(resultsSection).toHaveClass("fade-in");
      expect(resultsSection).toHaveClass("duration-1000");
    });
  });

  describe("Accessibility and ARIA attributes", () => {
    it("renders page header", () => {
      renderScanPage();
      expect(screen.getByRole("heading", { level: 1 })).toBeInTheDocument();
    });
  });
});