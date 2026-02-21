export type Severity = "critical" | "high" | "medium" | "low";

export interface SizeWarning {
  struct_name: string;
  estimated_size: number;
  limit: number;
  level?: "ExceedsLimit" | "ApproachingLimit";
}

export interface PanicIssue {
  function_name: string;
  issue_type: string;
  location: string;
}

export interface UnsafePattern {
  pattern_type: "Panic" | "Unwrap" | "Expect";
  line: number;
  snippet: string;
}

export interface ArithmeticIssue {
  function_name: string;
  operation: string;
  suggestion: string;
  location: string;
}

export interface CustomRuleMatch {
  rule_name: string;
  line: number;
  snippet: string;
}

export interface AnalysisReport {
  size_warnings?: SizeWarning[];
  unsafe_patterns?: UnsafePattern[];
  auth_gaps?: string[];
  panic_issues?: PanicIssue[];
  arithmetic_issues?: ArithmeticIssue[];
  custom_rule_matches?: CustomRuleMatch[];
}

export interface Finding {
  id: string;
  severity: Severity;
  category: string;
  title: string;
  location: string;
  snippet?: string;
  line?: number;
  suggestion?: string;
  raw: unknown;
}
