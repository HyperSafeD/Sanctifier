import { describe, it, expect } from "vitest";
import {
    canonicalizeFindingCode,
    normalizeFindingCodeQuery,
    validateFindingCodeQuery,
    filterFindings,
    FINDING_CODE_PATTERN,
} from "./finding-filters";
import { createFinding, createFindingList } from "../../tests/fixtures";

describe("Finding Filters", () => {
    describe("canonicalizeFindingCode", () => {
        it("returns valid S### codes unchanged", () => {
            expect(canonicalizeFindingCode("S001")).toBe("S001");
            expect(canonicalizeFindingCode("S999")).toBe("S999");
        });

        it("converts legacy codes to new format", () => {
            expect(canonicalizeFindingCode("AUTH_GAP")).toBe("S001");
            expect(canonicalizeFindingCode("PANIC_USAGE")).toBe("S002");
            expect(canonicalizeFindingCode("ARITHMETIC_OVERFLOW")).toBe("S003");
        });

        it("handles case-insensitive legacy codes", () => {
            expect(canonicalizeFindingCode("auth_gap")).toBe("S001");
            expect(canonicalizeFindingCode("Auth_Gap")).toBe("S001");
        });

        it("trims whitespace", () => {
            expect(canonicalizeFindingCode("  S001  ")).toBe("S001");
            expect(canonicalizeFindingCode("  AUTH_GAP  ")).toBe("S001");
        });

        it("returns empty string for empty input", () => {
            expect(canonicalizeFindingCode("")).toBe("");
            expect(canonicalizeFindingCode("   ")).toBe("");
        });

        it("returns unknown codes unchanged", () => {
            expect(canonicalizeFindingCode("UNKNOWN_CODE")).toBe("UNKNOWN_CODE");
        });
    });

    describe("normalizeFindingCodeQuery", () => {
        it("converts to uppercase", () => {
            expect(normalizeFindingCodeQuery("s001")).toBe("S001");
            expect(normalizeFindingCodeQuery("S001")).toBe("S001");
        });

        it("trims whitespace", () => {
            expect(normalizeFindingCodeQuery("  S001  ")).toBe("S001");
        });
    });

    describe("validateFindingCodeQuery", () => {
        it("accepts valid S### format", () => {
            expect(validateFindingCodeQuery("S001")).toBeNull();
            expect(validateFindingCodeQuery("S999")).toBeNull();
        });

        it("rejects invalid format", () => {
            const error = validateFindingCodeQuery("S00");
            expect(error).toBe("Use finding code format S### (for example: S001).");
        });

        it("rejects empty query", () => {
            expect(validateFindingCodeQuery("")).toBeNull();
        });

        it("rejects non-S### patterns", () => {
            expect(validateFindingCodeQuery("AUTH_GAP")).not.toBeNull();
            expect(validateFindingCodeQuery("001")).not.toBeNull();
        });
    });

    describe("FINDING_CODE_PATTERN", () => {
        it("matches valid codes", () => {
            expect(FINDING_CODE_PATTERN.test("S001")).toBe(true);
            expect(FINDING_CODE_PATTERN.test("S999")).toBe(true);
        });

        it("rejects invalid codes", () => {
            expect(FINDING_CODE_PATTERN.test("S00")).toBe(false);
            expect(FINDING_CODE_PATTERN.test("s001")).toBe(false);
            expect(FINDING_CODE_PATTERN.test("001")).toBe(false);
        });
    });

    describe("filterFindings", () => {
        const findings = [
            createFinding({ code: "S001", severity: "critical" }),
            createFinding({ code: "S002", severity: "high" }),
            createFinding({ code: "S003", severity: "medium" }),
            createFinding({ code: "S001", severity: "low" }),
        ];

        it("returns all findings when filter is 'all'", () => {
            const result = filterFindings(findings, "all", "");
            expect(result).toHaveLength(4);
        });

        it("filters by severity", () => {
            const result = filterFindings(findings, "critical", "");
            expect(result).toHaveLength(1);
            expect(result[0].severity).toBe("critical");
        });

        it("filters by code", () => {
            const result = filterFindings(findings, "all", "S001");
            expect(result).toHaveLength(2);
            expect(result.every((f) => f.code === "S001")).toBe(true);
        });

        it("filters by both severity and code", () => {
            const result = filterFindings(findings, "critical", "S001");
            expect(result).toHaveLength(1);
            expect(result[0].severity).toBe("critical");
            expect(result[0].code).toBe("S001");
        });

        it("returns empty array when no matches", () => {
            const result = filterFindings(findings, "critical", "S999");
            expect(result).toHaveLength(0);
        });

        it("handles empty findings array", () => {
            const result = filterFindings([], "all", "");
            expect(result).toHaveLength(0);
        });

        it("handles large finding lists efficiently", () => {
            const largeFindingList = createFindingList(1000, "medium");
            const result = filterFindings(largeFindingList, "medium", "");
            expect(result).toHaveLength(1000);
        });
    });
});
