import type { Finding, Severity } from "../../app/types";

export const mockFindings: Record<string, Finding> = {
    critical: {
        id: "finding-critical-1",
        code: "S001",
        title: "Authentication Gap",
        category: "Security",
        severity: "critical",
        location: "src/contract.rs:42",
        snippet: "pub fn transfer(to: Address, amount: i128) {",
        line: 42,
        suggestion: "Add require_auth() call before state modification",
        raw: null,
    },
    high: {
        id: "finding-high-1",
        code: "S002",
        title: "Panic Usage",
        category: "Reliability",
        severity: "high",
        location: "src/contract.rs:85",
        snippet: "panic!(\"Invalid amount\");",
        line: 85,
        suggestion: "Use Result type or custom error handling",
        raw: null,
    },
    medium: {
        id: "finding-medium-1",
        code: "S003",
        title: "Arithmetic Overflow",
        category: "Safety",
        severity: "medium",
        location: "src/math.rs:12",
        snippet: "let result = a + b;",
        line: 12,
        suggestion: "Use checked_add() or saturating_add()",
        raw: null,
    },
    low: {
        id: "finding-low-1",
        code: "S004",
        title: "Ledger Size Risk",
        category: "Performance",
        severity: "low",
        location: "src/storage.rs:99",
        snippet: "ledger.set(&key, &value);",
        line: 99,
        suggestion: "Implement storage limits or cleanup logic",
        raw: null,
    },
};

let findingCounter = 0;

export const createFinding = (overrides: Partial<Finding> = {}): Finding => {
    findingCounter++;
    return {
        id: `test-finding-${findingCounter}`,
        code: "S001",
        title: "Test Finding",
        category: "Test",
        severity: "medium" as Severity,
        location: "test.rs:1",
        raw: null,
        ...overrides,
    };
};

export const createFindingList = (count: number, severity?: Severity): Finding[] => {
    return Array.from({ length: count }, (_, i) => ({
        id: `finding-${Date.now()}-${i}`,
        code: `S${String(i + 1).padStart(3, "0")}`,
        title: `Finding ${i + 1}`,
        category: "Test",
        severity: severity || ("medium" as Severity),
        location: `test.rs:${i + 1}`,
        raw: null,
    }));
};
