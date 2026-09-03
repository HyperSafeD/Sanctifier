import { NextRequest } from "next/server";
import type { Finding } from "../../../types";
import { AI_EXPLAIN_PROVIDER } from "../../../lib/env";
import { logger } from "../../../lib/logger";

export async function POST(req: NextRequest) {
  const requestId = req.headers.get("x-request-id") || `req-${Math.random().toString(36).substring(2, 11)}`;
  logger.info("Received AI explanation request", {
    request_id: requestId,
    path: "/api/ai/explain",
    method: "POST",
  });

  if (!AI_EXPLAIN_PROVIDER && process.env.STUB_AI !== "1") {
    logger.warn("AI provider not configured", { request_id: requestId });
    return NextResponse.json(
      { error: "AI provider not configured", set: ["AI_EXPLAIN_PROVIDER"] },
      { status: 503, headers: { "x-request-id": requestId } }
    );
  }

  try {
    const { finding, stream } = (await req.json()) as { finding: Finding; stream?: boolean };

    if (!finding) {
      logger.warn("Missing finding parameter in AI explain request", { request_id: requestId });
      return NextResponse.json(
        { error: "Finding data is required" },
        { status: 400, headers: { "x-request-id": requestId } }
      );
    }

    // simulated delay
    await new Promise((resolve) => setTimeout(resolve, 1500));

    const explanations: Record<string, { explanation: string; fixCode: string }> = {
      "auth-gap": {
        explanation: "This finding indicates that a sensitive function is accessible without proper authorization checks. In Soroban, you should use `address.require_auth()` to ensure the caller is authorized to perform this action.",
        fixCode: `pub fn sensitive_action(env: Env, user: Address) {
    user.require_auth();
    // ... rest of the logic
}`,
      },
      "arithmetic-overflow": {
        explanation: "The code performs arithmetic operations that could result in an overflow if the values are too large. Soroban's `checked_add`, `checked_mul`, etc., should be used to handle these cases safely.",
        fixCode: `let result = a.checked_add(b).ok_or(Error::Overflow)?;`,
      },
      "storage-collision": {
        explanation: "Potential storage collision detected. Multiple contracts or functions might be using the same storage key, which can lead to data corruption or unauthorized access.",
        fixCode: `#[derive(Clone)]
#[repr(u32)]
pub enum DataKey {
    Admin = 1,
    Balance(Address) = 2,
}`,
      },
    };

    // Default response if category not matched
    const category = (finding.category || "").toLowerCase();
    const result = explanations[category] || {
      explanation: `The finding "${finding.title}" suggests a potential security risk in the contract logic. Specifically, at ${finding.location}, there is a concern regarding ${finding.category}. This could potentially be exploited to bypass security controls or cause unexpected behavior. We recommend implementing strict validation and following Soroban security best practices.`,
      fixCode: finding.suggestion ? `// Suggested Fix:\n// ${finding.suggestion}` : "// Review the logic at the specified location and add necessary checks.",
    };

    logger.info("Generated AI explanation successfully", {
      request_id: requestId,
      category: category,
    });
    return NextResponse.json(result, { headers: { "x-request-id": requestId } });
  } catch (err) {
    logger.error("AI explanation processing error", {
      request_id: requestId,
      error: err instanceof Error ? err.message : String(err),
    });
    return NextResponse.json(
      { error: "Internal server error" },
      { status: 500, headers: { "x-request-id": requestId } }
    );
  }
}
