//! `sanctifier benchmark` — standardised performance report.
//!
//! Runs every built-in rule against the `benchmarks/` contract corpus N times
//! (default 3) with the analysis cache cleared between rounds, then emits a
//! Markdown table with per-rule mean, p95 latency and a total.  Pass
//! `--baseline` to compare against a previously saved JSON snapshot.

use clap::Args;
use sanctifier_core::{Analyzer, SanctifyConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Args, Debug)]
pub struct BenchmarkArgs {
    /// Path to the contract corpus directory (defaults to `./benchmarks`)
    #[arg(default_value = "benchmarks")]
    pub corpus: PathBuf,

    /// Number of analysis iterations per rule (must be ≥ 1)
    #[arg(short = 'n', long, default_value_t = 3)]
    pub iterations: usize,

    /// Path to a baseline JSON file produced by a previous `--output` run.
    /// When provided, the table shows Δ columns relative to the baseline.
    #[arg(long)]
    pub baseline: Option<PathBuf>,

    /// Save the raw timing data as JSON to this path for future `--baseline` use.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// Per-rule timing data stored in the JSON snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTimings {
    pub rule: String,
    pub mean_ms: f64,
    pub p95_ms: f64,
    pub samples: Vec<f64>,
}

/// Full benchmark snapshot (written to `--output`).
#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkSnapshot {
    pub generated_at: String,
    pub corpus_path: String,
    pub iterations: usize,
    pub rules: Vec<RuleTimings>,
}

pub fn exec(args: BenchmarkArgs) -> anyhow::Result<()> {
    let iterations = args.iterations.max(1);

    // Collect all .rs source files from the corpus directory.
    let sources = collect_sources(&args.corpus)?;
    if sources.is_empty() {
        eprintln!(
            "No .rs files found in corpus path: {}",
            args.corpus.display()
        );
        eprintln!("Hint: pass a path to a directory containing Soroban contract .rs files.");
        return Ok(());
    }

    // Concatenate all sources into a single string so each rule is timed
    // against the full corpus in one pass.  This is intentionally simple and
    // mirrors how the analyser is used in CI.
    let combined: String = sources
        .iter()
        .filter_map(|p| fs::read_to_string(p).ok())
        .collect::<Vec<_>>()
        .join("\n\n");

    let config = SanctifyConfig::default();
    let analyzer = Analyzer::new(config);
    let rule_names: Vec<String> = analyzer
        .available_rules()
        .iter()
        .map(|s| s.to_string())
        .collect();

    eprintln!(
        "Running {} rules × {} iterations on {} file(s) from `{}`…",
        rule_names.len(),
        iterations,
        sources.len(),
        args.corpus.display()
    );

    // Time each rule individually.
    let mut timings: Vec<RuleTimings> = Vec::with_capacity(rule_names.len());
    for rule_name in &rule_names {
        let mut samples: Vec<f64> = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            // Re-create the analyzer each iteration to avoid any cached state.
            let a = Analyzer::new(SanctifyConfig::default());
            let start = Instant::now();
            let _ = a.run_rule(&combined, rule_name);
            samples.push(elapsed_ms(start.elapsed()));
        }
        let mean_ms = mean(&samples);
        let p95_ms = percentile(&mut samples.clone(), 95.0);
        timings.push(RuleTimings {
            rule: rule_name.clone(),
            mean_ms,
            p95_ms,
            samples,
        });
    }

    // Load baseline if provided.
    let baseline_map: HashMap<String, RuleTimings> = if let Some(ref path) = args.baseline {
        let raw = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read baseline {}: {}", path.display(), e))?;
        let snapshot: BenchmarkSnapshot = serde_json::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("Failed to parse baseline JSON: {}", e))?;
        snapshot
            .rules
            .into_iter()
            .map(|r| (r.rule.clone(), r))
            .collect()
    } else {
        HashMap::new()
    };

    // Render Markdown table.
    println!("{}", render_table(&timings, &baseline_map));

    // Persist snapshot if requested.
    if let Some(ref out_path) = args.output {
        let generated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());
        let snapshot = BenchmarkSnapshot {
            generated_at,
            corpus_path: args.corpus.display().to_string(),
            iterations,
            rules: timings,
        };
        let json = serde_json::to_string_pretty(&snapshot)?;
        fs::write(out_path, json)?;
        eprintln!("Saved benchmark snapshot to {}", out_path.display());
    }

    Ok(())
}

fn render_table(timings: &[RuleTimings], baseline: &HashMap<String, RuleTimings>) -> String {
    let has_baseline = !baseline.is_empty();
    let mut out = String::new();

    if has_baseline {
        out.push_str("| Rule | Mean ms | Δ Mean | p95 ms | Δ p95 |\n");
        out.push_str("|------|--------:|-------:|-------:|------:|\n");
    } else {
        out.push_str("| Rule | Mean ms | p95 ms |\n");
        out.push_str("|------|--------:|-------:|\n");
    }

    for t in timings {
        if has_baseline {
            if let Some(b) = baseline.get(&t.rule) {
                let d_mean = t.mean_ms - b.mean_ms;
                let d_p95 = t.p95_ms - b.p95_ms;
                out.push_str(&format!(
                    "| {} | {:.2} | {:+.2} | {:.2} | {:+.2} |\n",
                    t.rule, t.mean_ms, d_mean, t.p95_ms, d_p95
                ));
            } else {
                out.push_str(&format!(
                    "| {} | {:.2} | N/A | {:.2} | N/A |\n",
                    t.rule, t.mean_ms, t.p95_ms
                ));
            }
        } else {
            out.push_str(&format!(
                "| {} | {:.2} | {:.2} |\n",
                t.rule, t.mean_ms, t.p95_ms
            ));
        }
    }

    // Totals row
    let total_mean: f64 = timings.iter().map(|t| t.mean_ms).sum();
    let total_p95: f64 = timings.iter().map(|t| t.p95_ms).sum();
    if has_baseline {
        let base_mean: f64 = baseline.values().map(|b| b.mean_ms).sum();
        let base_p95: f64 = baseline.values().map(|b| b.p95_ms).sum();
        out.push_str(&format!(
            "| **Total** | **{:.2}** | **{:+.2}** | **{:.2}** | **{:+.2}** |\n",
            total_mean,
            total_mean - base_mean,
            total_p95,
            total_p95 - base_p95,
        ));
    } else {
        out.push_str(&format!(
            "| **Total** | **{:.2}** | **{:.2}** |\n",
            total_mean, total_p95
        ));
    }

    out
}

// ── timing helpers ────────────────────────────────────────────────────────────

fn elapsed_ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn mean(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().sum::<f64>() / samples.len() as f64
}

fn percentile(samples: &mut Vec<f64>, pct: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((pct / 100.0) * (samples.len() as f64 - 1.0)).round() as usize;
    samples[idx.min(samples.len() - 1)]
}

// ── file collection ───────────────────────────────────────────────────────────

fn collect_sources(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if !root.exists() {
        anyhow::bail!(
            "Corpus path `{}` does not exist. \
             Create a directory of Soroban .rs files or pass a different path.",
            root.display()
        );
    }
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }
    let mut files = Vec::new();
    collect_rs_files(root, &mut files);
    files.sort();
    Ok(files)
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() && !matches!(name, "target" | ".git") {
            collect_rs_files(&p, out);
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_and_percentile_basic() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((mean(&samples) - 3.0).abs() < 1e-9);
        let mut s = samples.clone();
        assert!((percentile(&mut s, 100.0) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn render_table_no_baseline() {
        let timings = vec![
            RuleTimings {
                rule: "auth_gap".to_string(),
                mean_ms: 1.23,
                p95_ms: 2.34,
                samples: vec![1.0, 1.23, 1.5],
            },
        ];
        let table = render_table(&timings, &HashMap::new());
        assert!(table.contains("auth_gap"));
        assert!(table.contains("1.23"));
        assert!(table.contains("2.34"));
        assert!(table.contains("Total"));
    }

    #[test]
    fn render_table_with_baseline() {
        let timings = vec![RuleTimings {
            rule: "panic_detection".to_string(),
            mean_ms: 2.0,
            p95_ms: 3.0,
            samples: vec![2.0],
        }];
        let mut baseline = HashMap::new();
        baseline.insert(
            "panic_detection".to_string(),
            RuleTimings {
                rule: "panic_detection".to_string(),
                mean_ms: 1.5,
                p95_ms: 2.5,
                samples: vec![1.5],
            },
        );
        let table = render_table(&timings, &baseline);
        assert!(table.contains("Δ Mean") || table.contains("Δ p95"));
        assert!(table.contains("+0.50") || table.contains("+0.5"));
    }
}
