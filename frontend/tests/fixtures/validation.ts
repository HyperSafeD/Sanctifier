export const mockFiles = {
    validRustFile: new File(
        ["pub fn main() {}"],
        "contract.rs",
        { type: "text/plain" }
    ),
    invalidExtension: new File(
        ["console.log('test')"],
        "script.js",
        { type: "text/javascript" }
    ),
    oversizedFile: new File(
        [new ArrayBuffer(300 * 1024)],
        "large.rs",
        { type: "text/plain" }
    ),
    emptyFile: new File([], "empty.rs", { type: "text/plain" }),
};

export const validationMessages = {
    invalidExtension: "Only .rs contract source files are supported.",
    oversizedFile: "File size exceeds 250 KB.",
};
