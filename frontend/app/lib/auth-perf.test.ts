import { describe, it, expect } from "vitest";
import { PERF_BUDGETS, measureAgainstBudget, timeAsync } from "./auth-perf";

// ─── measureAgainstBudget ─────────────────────────────────────────────────────

describe("measureAgainstBudget", () => {
  it("reports withinBudget=true when duration is under the budget", () => {
    const m = measureAgainstBudget("RATE_LIMIT_CHECK_MS", 1);
    expect(m.withinBudget).toBe(true);
    expect(m.budget).toBe(PERF_BUDGETS.RATE_LIMIT_CHECK_MS);
    expect(m.durationMs).toBe(1);
  });

  it("reports withinBudget=true when duration equals the budget exactly", () => {
    const m = measureAgainstBudget("RATE_LIMIT_CHECK_MS", PERF_BUDGETS.RATE_LIMIT_CHECK_MS);
    expect(m.withinBudget).toBe(true);
  });

  it("reports withinBudget=false when duration exceeds the budget", () => {
    const m = measureAgainstBudget("RATE_LIMIT_CHECK_MS", PERF_BUDGETS.RATE_LIMIT_CHECK_MS + 1);
    expect(m.withinBudget).toBe(false);
  });

  it("returns the correct label on the measurement", () => {
    const m = measureAgainstBudget("UPLOAD_VALIDATION_MS", 5);
    expect(m.label).toBe("UPLOAD_VALIDATION_MS");
  });
});

// ─── timeAsync ────────────────────────────────────────────────────────────────

describe("timeAsync", () => {
  it("returns the result of the wrapped function", async () => {
    const { result } = await timeAsync("RATE_LIMIT_CHECK_MS", async () => 42);
    expect(result).toBe(42);
  });

  it("produces a measurement with a non-negative duration", async () => {
    const { measurement } = await timeAsync("RATE_LIMIT_CHECK_MS", async () => {});
    expect(measurement.durationMs).toBeGreaterThanOrEqual(0);
  });

  it("marks a synchronously-instant operation as within budget", async () => {
    const { measurement } = await timeAsync("RATE_LIMIT_LOOKUP_MS", async () => "ok");
    expect(measurement.withinBudget).toBe(true);
  });
});

// ─── Access-control simulation benchmarks ─────────────────────────────────────

describe("rate-limit check is within RATE_LIMIT_CHECK_MS budget", () => {
  it("Map.get + Map.set completes within budget", async () => {
    const map = new Map<string, { count: number; resetTime: number }>();

    const { measurement } = await timeAsync("RATE_LIMIT_CHECK_MS", async () => {
      const now = Date.now();
      const entry = map.get("127.0.0.1");
      if (!entry || now > entry.resetTime) {
        map.set("127.0.0.1", { count: 1, resetTime: now + 60_000 });
      } else {
        entry.count++;
      }
    });

    expect(measurement.withinBudget).toBe(true);
  });
});

describe("upload validation is within UPLOAD_VALIDATION_MS budget", () => {
  it("extension check + size check completes within budget", async () => {
    const MAX_FILE_SIZE_BYTES = 250 * 1024;

    const { measurement } = await timeAsync("UPLOAD_VALIDATION_MS", async () => {
      const name = "contract.rs";
      const size = 1024;
      const ext = name.slice(name.lastIndexOf(".")).toLowerCase();
      return ext === ".rs" && size <= MAX_FILE_SIZE_BYTES;
    });

    expect(measurement.withinBudget).toBe(true);
  });
});
