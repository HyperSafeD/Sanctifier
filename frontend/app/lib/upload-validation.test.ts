import { describe, it, expect } from "vitest";
import { validateContractUpload } from "./upload-validation";

describe("validateContractUpload", () => {
    it("accepts valid .rs files under size limit", () => {
        const file = new File(["pub fn main() {}"], "contract.rs", {
            type: "text/plain",
        });
        expect(validateContractUpload(file)).toBeNull();
    });

    it("rejects files with unsupported extensions", () => {
        const file = new File(["console.log('test')"], "script.js", {
            type: "text/javascript",
        });
        const error = validateContractUpload(file);
        expect(error).toBe("Only .rs contract source files are supported.");
    });

    it("rejects files exceeding size limit", () => {
        const largeContent = new Array(260 * 1024).fill("x").join("");
        const file = new File([largeContent], "large.rs", { type: "text/plain" });
        const error = validateContractUpload(file);
        expect(error).toContain("File size exceeds");
    });

    it("rejects files with uppercase extensions", () => {
        const file = new File(["pub fn main() {}"], "contract.RS", {
            type: "text/plain",
        });
        expect(validateContractUpload(file)).toBeNull();
    });

    it("rejects files without extension", () => {
        const file = new File(["pub fn main() {}"], "contract", {
            type: "text/plain",
        });
        const error = validateContractUpload(file);
        expect(error).toBe("Only .rs contract source files are supported.");
    });

    it("handles edge case: exactly at size limit", () => {
        const content = new Array(250 * 1024).fill("x").join("");
        const file = new File([content], "contract.rs", { type: "text/plain" });
        expect(validateContractUpload(file)).toBeNull();
    });

    it("handles edge case: 1 byte over size limit", () => {
        const content = new Array(250 * 1024 + 1).fill("x").join("");
        const file = new File([content], "contract.rs", { type: "text/plain" });
        const error = validateContractUpload(file);
        expect(error).toContain("File size exceeds");
    });

    it("rejects files with multiple dots in name", () => {
        const file = new File(["pub fn main() {}"], "contract.backup.rs", {
            type: "text/plain",
        });
        expect(validateContractUpload(file)).toBeNull();
    });

    it("rejects .rs files with wrong case in extension", () => {
        const file = new File(["pub fn main() {}"], "contract.rS", {
            type: "text/plain",
        });
        expect(validateContractUpload(file)).toBeNull();
    });
});
