import type { AnalysisReport, CallGraphEdge, CallGraphNode, Finding, Severity } from "../types";

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

export function extractCallGraph(
  report: AnalysisReport
): { nodes: CallGraphNode[]; edges: CallGraphEdge[] } {
  const nodeMap = new Map<string, CallGraphNode>();
  const edges: CallGraphEdge[] = [];

  // Extract function nodes and storage mutation edges from auth gaps
  (report.auth_gaps ?? []).forEach((gap) => {
    // Auth gaps are strings like "file.rs:function_name" indicating functions
    // that mutate storage without authentication
    const parts = gap.split(":");
    const funcName = parts.length > 1 ? parts[parts.length - 1].trim() : gap;
    const file = parts.length > 1 ? parts.slice(0, -1).join(":").trim() : undefined;
    const funcId = `fn-${funcName}`;

    if (!nodeMap.has(funcId)) {
      nodeMap.set(funcId, {
        id: funcId,
        label: funcName,
        type: "function",
        file,
        severity: "critical",
      });
    }

    const storageId = `storage-${funcName}`;
    if (!nodeMap.has(storageId)) {
      nodeMap.set(storageId, {
        id: storageId,
        label: `${funcName} storage`,
        type: "storage",
      });
    }

    edges.push({
      source: funcId,
      target: storageId,
      label: "mutates (no auth)",
      type: "mutates",
    });
  });

  // Extract function nodes from panic issues
  (report.panic_issues ?? []).forEach((p) => {
    const funcId = `fn-${p.function_name}`;
    if (!nodeMap.has(funcId)) {
      nodeMap.set(funcId, {
        id: funcId,
        label: p.function_name,
        type: "function",
        severity: p.issue_type === "panic!" ? "critical" : "high",
      });
    }
  });

  // Extract function nodes from arithmetic issues
  (report.arithmetic_issues ?? []).forEach((a) => {
    const funcId = `fn-${a.function_name}`;
    if (!nodeMap.has(funcId)) {
      nodeMap.set(funcId, {
        id: funcId,
        label: a.function_name,
        type: "function",
        severity: "high",
      });
    }
  });

  return { nodes: Array.from(nodeMap.values()), edges };
}
