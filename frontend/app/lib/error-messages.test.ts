import { describe, it, expect } from "vitest";
import { errorMessages, getErrorMessage } from "./error-messages";

describe("Error Messages", () => {
    describe("errorMessages object", () => {
        it("contains upload validation messages", () => {
            expect(errorMessages.upload.unsupportedExtension).toBeDefined();
            expect(errorMessages.upload.fileTooLarge).toBeDefined();
            expect(errorMessages.upload.emptyFile).toBeDefined();
            expect(errorMessages.upload.invalidUtf8).toBeDefined();
        });

        it("contains finding code validation messages", () => {
            expect(errorMessages.findingCode.invalidFormat).toBeDefined();
            expect(errorMessages.findingCode.notFound).toBeDefined();
        });

        it("contains API error messages", () => {
            expect(errorMessages.api.rateLimited).toBeDefined();
            expect(errorMessages.api.timeout).toBeDefined();
            expect(errorMessages.api.invalidInput).toBeDefined();
            expect(errorMessages.api.payloadTooLarge).toBeDefined();
            expect(errorMessages.api.notSorobanContract).toBeDefined();
            expect(errorMessages.api.serverError).toBeDefined();
            expect(errorMessages.api.networkError).toBeDefined();
        });

        it("contains UI state messages", () => {
            expect(errorMessages.ui.noFindingsMatch).toBeDefined();
            expect(errorMessages.ui.loadingAnalysis).toBeDefined();
            expect(errorMessages.ui.analysisComplete).toBeDefined();
            expect(errorMessages.ui.errorOccurred).toBeDefined();
            expect(errorMessages.ui.tryAgain).toBeDefined();
        });

        it("contains validation messages", () => {
            expect(errorMessages.validation.required).toBeDefined();
            expect(errorMessages.validation.invalidFormat).toBeDefined();
        });
    });

    describe("getErrorMessage", () => {
        it("retrieves static messages", () => {
            const msg = getErrorMessage("upload.unsupportedExtension");
            expect(msg).toBe("Only .rs contract source files are supported.");
        });

        it("retrieves parameterized messages", () => {
            const msg = getErrorMessage("upload.fileTooLarge", { maxSizeKb: 250 });
            expect(msg).toContain("250");
        });

        it("retrieves nested messages", () => {
            const msg = getErrorMessage("api.timeout");
            expect(msg).toBe("Analysis timed out. Please try again.");
        });

        it("returns default message for invalid key", () => {
            const msg = getErrorMessage("invalid.key.path");
            expect(msg).toBe("An error occurred.");
        });

        it("handles rate limit message with seconds", () => {
            const msg = getErrorMessage("api.rateLimited", { retryAfterSeconds: 30 });
            expect(msg).toContain("30");
            expect(msg).toContain("seconds");
        });

        it("handles file size message with KB", () => {
            const msg = getErrorMessage("upload.fileTooLarge", { maxSizeKb: 500 });
            expect(msg).toContain("500");
            expect(msg).toContain("KB");
        });
    });
});
