import type {
  AnalysisReport,
  CallGraphEdge,
  CallGraphNode,
  CallGraphReportEdge,
  EventIssue,
  Finding,
  Severity,
  SmtIssue,
  StorageCollision,
  UnhandledResult,
  UpgradeFinding,
  VulnMatch,
} from "../types";
import { canonicalizeFindingCode } from "./finding-filters";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function arrayValue<T>(value: unknown): T[] {
  return Array.isArray(value) ? (value as T[]) : [];
}

export function normalizeReport(input: unknown): AnalysisReport {
  const parsed = isRecord(input) ? input : {};
  
  // Handle new WASM engine format (AnalysisResult with findings array)
  if (Array.isArray(parsed.findings)) {
    const findingsArray = parsed.findings as Array<Record<string, unknown>>;
    const report: AnalysisReport = {
      size_warnings: [],
      unsafe_patterns: [],
      auth_gaps: [],
      panic_issues: [],
      arithmetic_issues: [],
      storage_collisions: [],
      event_issues: [],
      unhandled_results: [],
      upgrade_reports: [],
      smt_violations: [],
      vuln_matches: [],
      call_graph: [],
    };

    findingsArray.forEach((f) => {
      const code = String(f.code || "");
      const category = String(f.category || "");
      const message = String(f.message || "");
      const location = String(f.location || "");

      if (category === "authentication") {
        report.auth_gaps!.push({ code, function_name: location });
      } else if (category === "panic_handling") {
        report.panic_issues!.push({ code, function_name: location, issue_type: "panic", location });
      } else if (category === "arithmetic") {
        report.arithmetic_issues!.push({ code, function_name: location, operation: message, suggestion: "", location });
      } else if (category === "storage_limits") {
        report.size_warnings!.push({ code, struct_name: location, estimated_size: 0, limit: 0, level: "ExceedsLimit" });
      } else {
        // Fallback to vuln_matches or similar
        report.vuln_matches!.push({ code, vuln_id: code, title: message, severity: "medium", location, description: message });
      }
    });

    return report;
  }

  // Handle legacy format (direct AnalysisReport structure)
  const findings = isRecord(parsed.findings) ? parsed.findings : parsed;
  const authGaps: AnalysisReport["auth_gaps"] = [];

  arrayValue<string | { function?: string; function_name?: string; code?: string }>(
    findings.auth_gaps
  ).forEach((gap) => {
    if (typeof gap === "string") {
      authGaps.push(gap);
      return;
    }

    if (!isRecord(gap)) {
      return;
    }

    const fnName =
      typeof gap.function_name === "string"
        ? gap.function_name
        : typeof gap.function === "string"
          ? gap.function
          : null;

    if (!fnName) {
      return;
    }

    authGaps.push({
      function_name: fnName,
      code: typeof gap.code === "string" ? gap.code : "AUTH_GAP",
    });
  });

  return {
    size_warnings: arrayValue(findings.size_warnings ?? findings.ledger_size_warnings),
    unsafe_patterns: arrayValue(findings.unsafe_patterns),
    auth_gaps: authGaps,
    panic_issues: arrayValue(findings.panic_issues),
    arithmetic_issues: arrayValue(findings.arithmetic_issues),
    custom_rule_matches: arrayValue(
      findings.custom_rule_matches ?? findings.custom_rules
    ),
    storage_collisions: arrayValue(findings.storage_collisions),
    event_issues: arrayValue(findings.event_issues),
    unhandled_results: arrayValue(findings.unhandled_results),
    upgrade_reports: arrayValue(findings.upgrade_reports),
    smt_violations: arrayValue(findings.smt_violations),
    vuln_matches: arrayValue(findings.vuln_matches),
    call_graph: arrayValue<CallGraphReportEdge>(parsed.call_graph).filter((edge) => {
      return isRecord(edge)
        && typeof edge.caller === "string"
        && typeof edge.callee === "string"
        && typeof edge.file === "string"
        && typeof edge.line === "number"
        && typeof edge.contract_id_expr === "string";
    }),
  };
}

function toFinding(
  id: string,
  code: string,
  severity: Severity,
  category: string,
  title: string,
  location: string,
  raw: unknown,
  opts?: { snippet?: string; line?: number; suggestion?: string }
): Finding {
  return {
    id,
    code: canonicalizeFindingCode(code),
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
    const location = typeof g === "string" ? g : g.function_name;
    const code = typeof g === "string" ? "AUTH_GAP" : g.code;
    findings.push(
      toFinding(
        `auth-${idx++}`,
        code,
        "critical",
        "Auth Gap",
        "Modifying state without require_auth()",
        location,
        g,
        { snippet: location }
      )
    );
  });

  (report.panic_issues ?? []).forEach((p) => {
    const severity: Severity = p.issue_type === "panic!" ? "critical" : "high";
    findings.push(
      toFinding(
        `panic-${idx++}`,
        p.code,
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
        a.code,
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
        w.code ?? "LEDGER_SIZE_RISK",
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
        u.code ?? "UNSAFE_PATTERN",
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
        m.code ?? "CUSTOM_RULE",
        "low",
        "Custom Rule",
        m.rule_name,
        m.snippet,
        m,
        { snippet: m.snippet, line: m.line }
      )
    );
  });

  (report.storage_collisions ?? []).forEach((s: StorageCollision) => {
    findings.push(
      toFinding(
        `storage-${idx++}`,
        s.code ?? "S005",
        "high",
        "Storage Collision",
        `Storage key collision on "${s.key}"`,
        s.location,
        s,
        { snippet: `Contracts: ${s.contracts.join(", ")}` }
      )
    );
  });

  (report.event_issues ?? []).forEach((e: EventIssue) => {
    findings.push(
      toFinding(
        `event-${idx++}`,
        e.code ?? "S008",
        "medium",
        "Event Issue",
        `${e.issue_type} in event "${e.event_name}"`,
        e.location,
        e,
        { suggestion: e.suggestion }
      )
    );
  });

  (report.unhandled_results ?? []).forEach((u: UnhandledResult) => {
    findings.push(
      toFinding(
        `unhandled-${idx++}`,
        u.code ?? "S009",
        "medium",
        "Unhandled Result",
        `Unhandled result from ${u.call_expr}`,
        u.location,
        u,
        { snippet: u.call_expr, suggestion: u.suggestion }
      )
    );
  });

  (report.upgrade_reports ?? []).forEach((up: UpgradeFinding) => {
    findings.push(
      toFinding(
        `upgrade-${idx++}`,
        up.code ?? "S010",
        "high",
        "Upgrade Issue",
        up.finding_type,
        up.location,
        up,
        { snippet: up.description, suggestion: up.suggestion }
      )
    );
  });

  (report.smt_violations ?? []).forEach((s: SmtIssue) => {
    findings.push(
      toFinding(
        `smt-${idx++}`,
        s.code ?? "S011",
        "critical",
        "SMT Violation",
        `Property violation: ${s.property}`,
        s.location,
        s,
        { snippet: s.violation, suggestion: s.suggestion }
      )
    );
  });

  (report.vuln_matches ?? []).forEach((v: VulnMatch) => {
    findings.push(
      toFinding(
        `vuln-${idx++}`,
        v.code ?? v.vuln_id,
        v.severity,
        "Vulnerability Match",
        v.title,
        v.location,
        v,
        { snippet: v.description }
      )
    );
  });

  return findings;
}

export function extractCallGraph(
  report: AnalysisReport
): { nodes: CallGraphNode[]; edges: CallGraphEdge[] } {
  if (report.call_graph && report.call_graph.length > 0) {
    return extractReportedCallGraph(report.call_graph);
  }

  const nodeMap = new Map<string, CallGraphNode>();
  const edges: CallGraphEdge[] = [];

  // Extract function nodes and storage mutation edges from auth gaps
  (report.auth_gaps ?? []).forEach((gap) => {
    // Auth gaps are strings like "file.rs:function_name" indicating functions
    // that mutate storage without authentication
    const location = typeof gap === "string" ? gap : gap.function_name;
    const parts = location.split(":");
    const funcName = parts.length > 1 ? parts[parts.length - 1].trim() : location;
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

  // Extract storage nodes from storage collisions
  (report.storage_collisions ?? []).forEach((s) => {
    const storageId = `storage-${s.key}`;
    if (!nodeMap.has(storageId)) {
      nodeMap.set(storageId, {
        id: storageId,
        label: `storage:${s.key}`,
        type: "storage",
        severity: "high",
      });
    }
  });

  // Extract function nodes from event issues
  (report.event_issues ?? []).forEach((e) => {
    const eventId = `fn-event-${e.event_name}`;
    if (!nodeMap.has(eventId)) {
      nodeMap.set(eventId, {
        id: eventId,
        label: e.event_name,
        type: "function",
        severity: "medium",
      });
    }
  });

  return { nodes: Array.from(nodeMap.values()), edges };
}

function extractReportedCallGraph(
  reportedEdges: CallGraphReportEdge[]
): { nodes: CallGraphNode[]; edges: CallGraphEdge[] } {
  const nodeMap = new Map<string, CallGraphNode>();
  const edges: CallGraphEdge[] = [];

  // First pass: collect all known caller names so we can classify edges.
  const knownCallers = new Set(reportedEdges.map((e) => e.caller));

  reportedEdges.forEach((edge) => {
    const sourceId = `fn-${edge.caller}`;
    // If the callee is also a known caller in this project it's an internal call.
    const isInternal = knownCallers.has(edge.callee);
    const targetId = isInternal ? `fn-${edge.callee}` : `external-${edge.callee}`;

    if (!nodeMap.has(sourceId)) {
      nodeMap.set(sourceId, {
        id: sourceId,
        label: edge.caller,
        type: "function",
        file: edge.file,
      });
    }

    if (!nodeMap.has(targetId)) {
      nodeMap.set(targetId, {
        id: targetId,
        label: edge.callee,
        type: isInternal ? "function" : "external",
      });
    }

    edges.push({
      source: sourceId,
      target: targetId,
      label: edge.function_expr
        ? `${edge.function_expr} (${edge.file}:${edge.line})`
        : `${edge.file}:${edge.line}`,
      type: isInternal ? "internal" : "calls",
    });
  });

  return { nodes: Array.from(nodeMap.values()), edges };
}
