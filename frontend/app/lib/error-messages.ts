/**
 * Centralized error messages for i18n readiness.
 * All user-facing error messages are defined here for easy translation.
 */

export const errorMessages = {
    // Upload validation errors
    upload: {
        unsupportedExtension: "Only .rs contract source files are supported.",
        fileTooLarge: (maxSizeKb: number) =>
            `File size exceeds ${maxSizeKb} KB.`,
        emptyFile: "File cannot be empty.",
        invalidUtf8: "File must be valid UTF-8 encoded text.",
    },

    // Finding code validation errors
    findingCode: {
        invalidFormat: "Use finding code format S### (for example: S001).",
        notFound: "Finding code not found in results.",
    },

    // API errors
    api: {
        rateLimited: (retryAfterSeconds: number) =>
            `Too many requests. Please try again in ${retryAfterSeconds} seconds.`,
        timeout: "Analysis timed out. Please try again.",
        invalidInput: "Invalid input provided.",
        payloadTooLarge: "Request payload is too large.",
        notSorobanContract: "File does not appear to be a Soroban contract.",
        serverError: "Server error occurred. Please try again later.",
        networkError: "Network error. Please check your connection.",
    },

    // UI state messages
    ui: {
        noFindingsMatch: "No findings match the selected filter.",
        loadingAnalysis: "Analyzing contract...",
        analysisComplete: "Analysis complete.",
        errorOccurred: "Something went wrong",
        tryAgain: "Try again",
    },

    // Validation messages
    validation: {
        required: "This field is required.",
        invalidFormat: "Invalid format.",
    },
} as const;

/**
 * Get error message with optional parameters.
 * Supports both static and parameterized messages.
 */
export function getErrorMessage(
    key: string,
    params?: Record<string, string | number>
): string {
    const keys = key.split(".");
    let message: unknown = errorMessages;

    for (const k of keys) {
        message = (message as Record<string, unknown>)?.[k];
    }

    if (typeof message === "function") {
        return message(...Object.values(params || {}));
    }

    return typeof message === "string" ? message : "An error occurred.";
}
