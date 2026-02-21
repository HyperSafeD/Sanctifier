import type { AnalysisReport, Finding, Severity } from "../types";

function toFinding(
  id: string,
  severity: Severity,
  category: string,
  title: string,
  location: string,
  raw: unknown,
  opts?: { snippet?: string; line?: number; suggestion?: string }
): Finding {
  return {
    id,
    severity,
    category,
    title,
    location,
    raw,
    ...opts,
  };
}

export function transformReport(report: AnalysisReport): Finding[] {
  const findings: Finding[] = [];
  let idx = 0;

  (report.auth_gaps ?? []).forEach((g) => {
    findings.push(
      toFinding(
        `auth-${idx++}`,
        "critical",
        "Auth Gap",
        "Modifying state without require_auth()",
        g,
        { snippet: g }
      )
    );
  });

  (report.panic_issues ?? []).forEach((p) => {
    const severity: Severity = p.issue_type === "panic!" ? "critical" : "high";
    findings.push(
      toFinding(
        `panic-${idx++}`,
        severity,
        "Panic/Unwrap",
        `Using ${p.issue_type}`,
        p.location,
        p,
        { snippet: p.function_name }
      )
    );
  });

  (report.arithmetic_issues ?? []).forEach((a) => {
    findings.push(
      toFinding(
        `arith-${idx++}`,
        "high",
        "Arithmetic",
        `Unchecked ${a.operation}`,
        a.location,
        a,
        { snippet: a.operation, suggestion: a.suggestion }
      )
    );
  });

  (report.size_warnings ?? []).forEach((w) => {
    const severity: Severity = w.level === "ExceedsLimit" ? "high" : "medium";
    findings.push(
      toFinding(
        `size-${idx++}`,
        severity,
        "Ledger Size",
        `Struct ${w.struct_name} ${w.level === "ExceedsLimit" ? "exceeds" : "approaching"} limit`,
        w.struct_name,
        w,
        { snippet: `${w.estimated_size} bytes (limit: ${w.limit})` }
      )
    );
  });

  (report.unsafe_patterns ?? []).forEach((u) => {
    findings.push(
      toFinding(
        `unsafe-${idx++}`,
        "medium",
        "Unsafe Pattern",
        u.pattern_type,
        u.snippet,
        u,
        { snippet: u.snippet, line: u.line }
      )
    );
  });

  (report.custom_rule_matches ?? []).forEach((m) => {
    findings.push(
      toFinding(
        `custom-${idx++}`,
        "low",
        "Custom Rule",
        m.rule_name,
        m.snippet,
        m,
        { snippet: m.snippet, line: m.line }
      )
    );
  });

  return findings;
}
