// tooling/sanctifier-core/src/gas_report.rs
//
// Aggregated gas/instruction report for the Sanctifier CLI.
//
// This module wraps ``gas_estimator`` output and provides human-readable
// text and JSON rendering used by `sanctifier gas` subcommand.

use crate::gas_estimator::GasEstimationReport;
use serde::Serialize;

// ── Report types ──────────────────────────────────────────────────────────────

/// Severity tier based on estimated instruction count.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum GasTier {
    /// < 10 000 instructions — well within network limits.
    Low,
    /// 10 000 – 99 999 instructions — review recommended.
    Medium,
    /// ≥ 100 000 instructions — likely to hit resource limits.
    High,
}

impl GasTier {
    pub fn from_instructions(n: usize) -> Self {
        if n >= 100_000 {
            GasTier::High
        } else if n >= 10_000 {
            GasTier::Medium
        } else {
            GasTier::Low
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            GasTier::Low => "LOW",
            GasTier::Medium => "MEDIUM",
            GasTier::High => "HIGH",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            GasTier::Low => "✅",
            GasTier::Medium => "⚠️ ",
            GasTier::High => "🔴",
        }
    }
}

/// A single annotated function entry in the gas report.
#[derive(Debug, Serialize, Clone)]
pub struct GasReportEntry {
    pub function_name: String,
    pub estimated_instructions: usize,
    pub estimated_memory_bytes: usize,
    pub tier: GasTier,
}

impl From<GasEstimationReport> for GasReportEntry {
    fn from(r: GasEstimationReport) -> Self {
        let tier = GasTier::from_instructions(r.estimated_instructions);
        GasReportEntry {
            function_name: r.function_name,
            estimated_instructions: r.estimated_instructions,
            estimated_memory_bytes: r.estimated_memory_bytes,
            tier,
        }
    }
}

/// Full gas report for one or more files.
#[derive(Debug, Serialize, Clone)]
pub struct GasReport {
    pub entries: Vec<GasReportEntry>,
    pub total_instructions: usize,
    pub total_memory_bytes: usize,
}

impl GasReport {
    pub fn from_estimations(reports: Vec<GasEstimationReport>) -> Self {
        let entries: Vec<GasReportEntry> = reports.into_iter().map(Into::into).collect();
        let total_instructions = entries.iter().map(|e| e.estimated_instructions).sum();
        let total_memory_bytes = entries.iter().map(|e| e.estimated_memory_bytes).sum();
        GasReport {
            entries,
            total_instructions,
            total_memory_bytes,
        }
    }
}

// ── Text rendering ────────────────────────────────────────────────────────────

/// Render a human-readable console report.
pub fn render_text(report: &GasReport) -> String {
    let mut out = String::new();
    out.push_str("╔══════════════════════════════════════════════════════════════╗\n");
    out.push_str("║     ⛽  SANCTIFIER — GAS ESTIMATION REPORT                   ║\n");
    out.push_str("╚══════════════════════════════════════════════════════════════╝\n\n");

    if report.entries.is_empty() {
        out.push_str("  No public contract functions found.\n");
        return out;
    }

    out.push_str("┌────────────────────────┬─────────────┬────────────┬────────┐\n");
    out.push_str("│ Function               │ Instructions│ Memory (B) │ Tier   │\n");
    out.push_str("├────────────────────────┼─────────────┼────────────┼────────┤\n");

    for e in &report.entries {
        out.push_str(&format!(
            "│ {:<22} │ {:>11} │ {:>10} │ {} {:<4} │\n",
            truncate(&e.function_name, 22),
            e.estimated_instructions,
            e.estimated_memory_bytes,
            e.tier.emoji(),
            e.tier.label(),
        ));
    }
    out.push_str("└────────────────────────┴─────────────┴────────────┴────────┘\n\n");
    out.push_str(&format!(
        "  Total  instructions : {}\n  Total  memory       : {} bytes\n",
        report.total_instructions, report.total_memory_bytes
    ));
    out.push_str("\n  Tiers: LOW < 10k | MEDIUM 10k–99k | HIGH ≥ 100k instructions\n");
    out
}

/// Render as pretty-printed JSON.
pub fn render_json(report: &GasReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gas_estimator::GasEstimationReport;

    #[test]
    fn test_tier_classification() {
        assert_eq!(GasTier::from_instructions(0), GasTier::Low);
        assert_eq!(GasTier::from_instructions(9_999), GasTier::Low);
        assert_eq!(GasTier::from_instructions(10_000), GasTier::Medium);
        assert_eq!(GasTier::from_instructions(99_999), GasTier::Medium);
        assert_eq!(GasTier::from_instructions(100_000), GasTier::High);
    }

    #[test]
    fn test_report_from_estimations() {
        let raw = vec![
            GasEstimationReport {
                function_name: "transfer".to_string(),
                estimated_instructions: 1500,
                estimated_memory_bytes: 256,
            },
            GasEstimationReport {
                function_name: "batch_transfer".to_string(),
                estimated_instructions: 150_000,
                estimated_memory_bytes: 4096,
            },
        ];
        let report = GasReport::from_estimations(raw);
        assert_eq!(report.entries.len(), 2);
        assert_eq!(report.entries[0].tier, GasTier::Low);
        assert_eq!(report.entries[1].tier, GasTier::High);
        assert_eq!(report.total_instructions, 151_500);
    }

    #[test]
    fn test_render_text_nonempty() {
        let report = GasReport::from_estimations(vec![GasEstimationReport {
            function_name: "mint".to_string(),
            estimated_instructions: 500,
            estimated_memory_bytes: 64,
        }]);
        let text = render_text(&report);
        assert!(text.contains("mint"));
        assert!(text.contains("LOW"));
    }
}
