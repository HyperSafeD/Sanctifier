use clap::Args;
use colored::Colorize;
use sanctifier_core::finding_codes::{self, FindingCode, FindingSeverity};

#[derive(Args, Debug)]
pub struct ExplainArgs {
    /// Finding code to explain (e.g. S001, S003)
    #[arg(required_unless_present = "interactive")]
    pub code: Option<String>,

    /// Output format: text | json
    #[arg(long, default_value = "text")]
    pub format: String,

    /// Interactive code browser with fuzzy search
    #[arg(short, long)]
    pub interactive: bool,
}

pub fn exec(args: ExplainArgs) -> anyhow::Result<()> {
    if args.interactive {
        return exec_interactive();
    }

    let code_str = args.code.as_deref().unwrap_or("");
    let all_codes = finding_codes::all_finding_codes();

    let finding = lookup_code(code_str, &all_codes);

    match finding {
        Some(fc) => {
            if args.format == "json" {
                println!("{}", serde_json::to_string_pretty(fc)?);
            } else {
                print_finding(fc);
            }
            Ok(())
        }
        None => {
            let similar = fuzzy_find_similar(code_str, &all_codes);
            if similar.is_empty() {
                anyhow::bail!(
                    "Unknown finding code: '{}'. \
                     Use 'sanctifier explain --interactive' to browse available codes.",
                    code_str
                );
            }
            eprintln!("{} Unknown finding code: '{}'", "✘".red().bold(), code_str);
            eprintln!();
            eprintln!("Did you mean one of these?");
            for s in &similar {
                eprintln!("  {} - {}", s.code.bold(), s.title);
            }
            anyhow::bail!("No exact match found for '{}'", code_str);
        }
    }
}

fn lookup_code<'a>(code_str: &str, codes: &'a [FindingCode]) -> Option<&'a FindingCode> {
    let trimmed = code_str.trim().to_uppercase();

    let exact = codes.iter().find(|c| c.code == trimmed);
    if exact.is_some() {
        return exact;
    }

    let lowered = trimmed.to_lowercase();
    let mut scored: Vec<(&FindingCode, i32)> = codes
        .iter()
        .filter_map(|c| {
            let mut score = 0i32;
            let code_lower = c.code.to_lowercase();
            let title_lower = c.title.to_lowercase();
            let cat_lower = c.category.to_lowercase();

            if code_lower == lowered {
                score += 100;
            }
            if code_lower.contains(&lowered) {
                score += 50;
            }
            if title_lower.contains(&lowered) {
                score += 30;
            }
            if cat_lower.contains(&lowered) {
                score += 20;
            }
            if lowered.contains(code_lower.trim_start_matches('s'))
                || lowered.contains(code_lower.trim_start_matches('S'))
            {
                score += 10;
            }

            if score > 0 {
                Some((c, score))
            } else {
                None
            }
        })
        .collect();

    scored.sort_by_key(|k| std::cmp::Reverse(k.1));
    scored.into_iter().next().map(|(c, _)| c)
}

fn fuzzy_find_similar<'a>(code_str: &str, codes: &'a [FindingCode]) -> Vec<&'a FindingCode> {
    let lowered = code_str.to_lowercase();
    let mut scored: Vec<(&FindingCode, i32)> = codes
        .iter()
        .filter_map(|c| {
            let mut score = 0i32;
            let code_lower = c.code.to_lowercase();
            let title_lower = c.title.to_lowercase();

            if code_lower == lowered {
                score += 100;
            } else if code_lower.contains(&lowered) || lowered.contains(&code_lower) {
                score += 40;
            }
            if title_lower.contains(&lowered) || lowered.contains(&title_lower) {
                score += 20;
            }

            if score > 0 {
                Some((c, score))
            } else {
                None
            }
        })
        .collect();

    scored.sort_by_key(|k| std::cmp::Reverse(k.1));
    scored.into_iter().take(5).map(|(c, _)| c).collect()
}

fn print_finding(fc: &FindingCode) {
    let severity_label = match fc.severity {
        FindingSeverity::Critical => "CRITICAL".red().bold().to_string(),
        FindingSeverity::High => "HIGH".yellow().bold().to_string(),
        FindingSeverity::Medium => "MEDIUM".yellow().to_string(),
        FindingSeverity::Low => "LOW".dimmed().to_string(),
        FindingSeverity::Info => "INFO".cyan().to_string(),
    };

    println!();
    println!("{}  {} — {}", "◆".bold(), fc.code.bold(), fc.title.bold());
    println!("{}", "━".repeat(60));
    println!("  {}   {}", "Severity:".dimmed(), severity_label);
    println!("  {}   {}", "Category:".dimmed(), fc.category);
    println!();
    println!("{}", fc.description);
    println!();
    println!("{}", "Remediation".bold().underline());
    println!("{}", fc.remediation);
    println!();
    println!("{}", "Documentation".bold().underline());
    println!("{}", fc.doc_url);
    println!();
}

fn exec_interactive() -> anyhow::Result<()> {
    let all_codes = finding_codes::all_finding_codes();
    let selections: Vec<String> = all_codes
        .iter()
        .map(|c| format!("{}  {}  {}", c.code, severity_symbol(c.severity), c.title))
        .collect();

    let selection = dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("🔍 Search finding codes (type to filter, ↑↓ to navigate, Enter to select)")
        .items(&selections)
        .default(0)
        .interact()
        .map_err(|e| anyhow::anyhow!("Interactive selection failed: {}", e))?;

    let selected = &all_codes[selection];
    print_finding(selected);
    Ok(())
}

fn severity_symbol(severity: FindingSeverity) -> &'static str {
    match severity {
        FindingSeverity::Critical => "🔴",
        FindingSeverity::High => "🟠",
        FindingSeverity::Medium => "🟡",
        FindingSeverity::Low => "🔵",
        FindingSeverity::Info => "⚪",
    }
}
