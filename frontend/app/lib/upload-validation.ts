import { errorMessages } from "./error-messages";

const SUPPORTED_CONTRACT_EXTENSIONS = new Set([".rs"]);
const MAX_CONTRACT_UPLOAD_SIZE_BYTES = 250 * 1024;

function getFileExtension(name: string): string {
  const idx = name.lastIndexOf(".");
  return idx >= 0 ? name.slice(idx).toLowerCase() : "";
}

export function validateContractUpload(file: File): string | null {
  if (file.size > MAX_CONTRACT_UPLOAD_SIZE_BYTES) {
    return errorMessages.upload.fileTooLarge(MAX_CONTRACT_UPLOAD_SIZE_BYTES / 1024);
  }

  const extension = getFileExtension(file.name);
  if (!SUPPORTED_CONTRACT_EXTENSIONS.has(extension)) {
    return errorMessages.upload.unsupportedExtension;
  }

  return null;
}

export type RejectedFile = { name: string; reason: string };

export type BatchValidationResult = {
  valid: File[];
  rejected: RejectedFile[];
};

export function validateContractBatch(files: File[]): BatchValidationResult {
  const valid: File[] = [];
  const rejected: RejectedFile[] = [];
  for (const file of files) {
    const error = validateContractUpload(file);
    if (error) {
      rejected.push({ name: file.name, reason: error });
    } else {
      valid.push(file);
    }
  }
  return { valid, rejected };
}
