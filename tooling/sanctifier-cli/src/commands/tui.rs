use crate::commands::analyze::{
    analyze_single_file, collect_rs_files, is_soroban_project, load_config, run_with_timeout,
    FileAnalysisResult,
};
use crate::vulndb::{VulnDatabase, VulnMatch};
use anyhow::Context;
use clap::Args;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap};
use ratatui::{Frame, Terminal};
use rayon::prelude::*;
use sanctifier_core::finding_codes;
use sanctifier_core::{Analyzer, SizeWarningLevel};
use std::fs;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::warn;

#[derive(Args, Debug)]
pub struct TuiArgs {
    /// Path to the contract directory or a single `.rs` file
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Ledger entry size limit in bytes
    #[arg(short, long, default_value = "64000")]
    pub limit: usize,

    /// Path to a custom vulnerability database JSON file
    #[arg(long)]
    pub vuln_db: Option<PathBuf>,

    /// Per-file analysis timeout in seconds (0 = disabled)
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum DashboardSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl DashboardSeverity {
    fn label(self) -> &'static str {
        match self {
            DashboardSeverity::Critical => "Critical",
            DashboardSeverity::High => "High",
            DashboardSeverity::Medium => "Medium",
            DashboardSeverity::Low => "Low",
            DashboardSeverity::Info => "Info",
        }
    }

    fn color(self) -> Color {
        match self {
            DashboardSeverity::Critical => Color::Red,
            DashboardSeverity::High => Color::LightRed,
            DashboardSeverity::Medium => Color::Yellow,
            DashboardSeverity::Low => Color::Cyan,
            DashboardSeverity::Info => Color::Blue,
        }
    }
}

struct DashboardSection {
    title: String,
    code: String,
    severity: DashboardSeverity,
    items: Vec<String>,
}

impl DashboardSection {
    fn count(&self) -> usize {
        self.items.len()
    }
}

struct DashboardData {
    path: String,
    total_files: usize,
    total_findings: usize,
    highest_severity: DashboardSeverity,
    duration_ms: u64,
    vuln_db_version: String,
    sections: Vec<DashboardSection>,
}

struct DashboardApp {
    data: DashboardData,
    active_tab: usize,
    selected_item: usize,
}

impl DashboardApp {
    fn new(data: DashboardData) -> Self {
        Self {
            data,
            active_tab: 0,
            selected_item: 0,
        }
    }

    fn current_section(&self) -> &DashboardSection {
        &self.data.sections[self.active_tab]
    }

    fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.data.sections.len();
        self.selected_item = 0;
    }

    fn previous_tab(&mut self) {
        self.active_tab = if self.active_tab == 0 {
            self.data.sections.len() - 1
        } else {
            self.active_tab - 1
        };
        self.selected_item = 0;
    }

    fn next_item(&mut self) {
        let len = self.current_section().items.len();
        if len > 0 {
            self.selected_item = (self.selected_item + 1) % len;
        }
    }

    fn previous_item(&mut self) {
        let len = self.current_section().items.len();
        if len > 0 {
            self.selected_item = if self.selected_item == 0 {
                len - 1
            } else {
                self.selected_item - 1
            };
        }
    }
}

pub fn exec(args: TuiArgs) -> anyhow::Result<()> {
    let data = build_dashboard_data(&args)?;

    if !io::stdout().is_terminal() {
        println!("{}", render_snapshot(&data));
        return Ok(());
    }

    run_terminal_dashboard(data)
}

fn build_dashboard_data(args: &TuiArgs) -> anyhow::Result<DashboardData> {
    let mut path = args.path.clone();

    #[cfg(not(windows))]
    {
        let as_text = path.to_string_lossy();
        if as_text.contains('\\') {
            path = PathBuf::from(as_text.replace('\\', "/"));
        }
    }

    if !is_soroban_project(&path) {
        anyhow::bail!(
            "{:?} is not a valid Soroban project (no Cargo.toml with soroban-sdk found)",
            path
        );
    }

    let start = Instant::now();
    let mut config = load_config(&path);
    config.ledger_limit = args.limit;
    let analyzer = Arc::new(Analyzer::new(config));
    let vuln_db = Arc::new(match &args.vuln_db {
        Some(db_path) => VulnDatabase::load(db_path)?,
        None => VulnDatabase::load_default(),
    });

    let rs_files = if path.is_dir() {
        collect_rs_files(&path, &analyzer.config.ignore_paths)
    } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
        vec![path.clone()]
    } else {
        Vec::new()
    };

    let total_files = rs_files.len();
    let counter = Arc::new(AtomicUsize::new(0));
    let timeout_dur = if args.timeout == 0 {
        None
    } else {
        Some(Duration::from_secs(args.timeout))
    };

    let mut results: Vec<FileAnalysisResult> = rs_files
        .par_iter()
        .map(|file_path| {
            let idx = counter.fetch_add(1, Ordering::Relaxed) + 1;
            eprintln!(
                "[{}/{}] Preparing dashboard for {}",
                idx,
                total_files,
                file_path.display()
            );

            let content = match fs::read_to_string(file_path) {
                Ok(content) => content,
                Err(_) => return FileAnalysisResult::default(),
            };

            let analyzer = Arc::clone(&analyzer);
            let vuln_db = Arc::clone(&vuln_db);
            let file_name = file_path.display().to_string();
            let file_name_clone = file_name.clone();

            match run_with_timeout(timeout_dur, move || {
                analyze_single_file(&analyzer, &vuln_db, &content, &file_name_clone)
            }) {
                Some(result) => result,
                None => {
                    warn!(
                        target: "sanctifier",
                        file = %file_name,
                        timeout_secs = args.timeout,
                        "Dashboard analysis timed out"
                    );
                    FileAnalysisResult {
                        file_path: file_name,
                        timed_out: true,
                        ..Default::default()
                    }
                }
            }
        })
        .collect();

    results.sort_by(|left, right| left.file_path.cmp(&right.file_path));

    let mut sections = vec![
        DashboardSection {
            title: "Authentication Gaps".into(),
            code: finding_codes::AUTH_GAP.into(),
            severity: DashboardSeverity::Critical,
            items: Vec::new(),
        },
        DashboardSection {
            title: "Panics".into(),
            code: finding_codes::PANIC_USAGE.into(),
            severity: DashboardSeverity::High,
            items: Vec::new(),
        },
        DashboardSection {
            title: "Arithmetic".into(),
            code: finding_codes::ARITHMETIC_OVERFLOW.into(),
            severity: DashboardSeverity::High,
            items: Vec::new(),
        },
        DashboardSection {
            title: "Ledger Size".into(),
            code: finding_codes::LEDGER_SIZE_RISK.into(),
            severity: DashboardSeverity::Medium,
            items: Vec::new(),
        },
        DashboardSection {
            title: "Storage".into(),
            code: finding_codes::STORAGE_COLLISION.into(),
            severity: DashboardSeverity::High,
            items: Vec::new(),
        },
        DashboardSection {
            title: "Unsafe Patterns".into(),
            code: finding_codes::UNSAFE_PATTERN.into(),
            severity: DashboardSeverity::Medium,
            items: Vec::new(),
        },
        DashboardSection {
            title: "Events".into(),
            code: finding_codes::EVENT_INCONSISTENCY.into(),
            severity: DashboardSeverity::Low,
            items: Vec::new(),
        },
        DashboardSection {
            title: "Unhandled Results".into(),
            code: finding_codes::UNHANDLED_RESULT.into(),
            severity: DashboardSeverity::High,
            items: Vec::new(),
        },
        DashboardSection {
            title: "Upgrade Risks".into(),
            code: finding_codes::UPGRADE_RISK.into(),
            severity: DashboardSeverity::High,
            items: Vec::new(),
        },
        DashboardSection {
            title: "SMT".into(),
            code: finding_codes::SMT_INVARIANT_VIOLATION.into(),
            severity: DashboardSeverity::Critical,
            items: Vec::new(),
        },
        DashboardSection {
            title: "SEP-41".into(),
            code: finding_codes::SEP41_INTERFACE_DEVIATION.into(),
            severity: DashboardSeverity::Medium,
            items: Vec::new(),
        },
        DashboardSection {
            title: "Known Vulns".into(),
            code: "VDB".into(),
            severity: DashboardSeverity::High,
            items: Vec::new(),
        },
        DashboardSection {
            title: "Timeouts".into(),
            code: finding_codes::ANALYSIS_TIMEOUT.into(),
            severity: DashboardSeverity::Info,
            items: Vec::new(),
        },
    ];

    for result in results {
        sections[0].items.extend(
            result
                .auth_gaps
                .into_iter()
                .map(|gap| format!("Function {}", gap.function_name)),
        );
        sections[1].items.extend(
            result
                .panic_issues
                .into_iter()
                .map(|issue| format!("{} at {}", issue.issue_type, issue.location)),
        );
        sections[2].items.extend(
            result
                .arithmetic_issues
                .into_iter()
                .map(|issue| format!("{} at {}", issue.operation, issue.location)),
        );
        sections[3]
            .items
            .extend(
                result
                    .size_warnings
                    .into_iter()
                    .map(|warning| match warning.level {
                        SizeWarningLevel::ExceedsLimit => {
                            format!(
                                "{} estimated at {} bytes",
                                warning.struct_name, warning.estimated_size
                            )
                        }
                        SizeWarningLevel::NearLimit => {
                            format!(
                                "{} near limit at {} bytes",
                                warning.struct_name, warning.estimated_size
                            )
                        }
                    }),
            );
        sections[4]
            .items
            .extend(result.collisions.into_iter().map(|collision| {
                format!(
                    "{} [{}] at {}",
                    collision.key_value, collision.key_type, collision.location
                )
            }));
        sections[5].items.extend(
            result
                .unsafe_patterns
                .into_iter()
                .map(|pattern| pattern.snippet),
        );
        sections[6]
            .items
            .extend(result.event_issues.into_iter().map(|issue| {
                format!(
                    "{} {:?} at {}",
                    issue.event_name, issue.issue_type, issue.location
                )
            }));
        sections[7]
            .items
            .extend(result.unhandled_results.into_iter().map(|issue| {
                format!(
                    "{} ignored {} at {}",
                    issue.function_name, issue.call_expression, issue.location
                )
            }));
        sections[8]
            .items
            .extend(result.upgrade_reports.into_iter().flat_map(|report| {
                report.findings.into_iter().map(|finding| {
                    format!(
                        "{:?} at {}: {}",
                        finding.category, finding.location, finding.message
                    )
                })
            }));
        sections[9]
            .items
            .extend(result.smt_issues.into_iter().map(|issue| {
                format!(
                    "{} at {}: {}",
                    issue.function_name, issue.location, issue.description
                )
            }));
        sections[10]
            .items
            .extend(result.sep41_issues.into_iter().map(|issue| {
                format!(
                    "{} {:?} at {}",
                    issue.function_name, issue.kind, issue.location
                )
            }));
        sections[11]
            .items
            .extend(result.vuln_matches.into_iter().map(format_vuln_match));
        if result.timed_out {
            sections[12]
                .items
                .push(format!("{} exceeded {}s", result.file_path, args.timeout));
        }
    }

    if !sections[11].items.is_empty() {
        sections[11].severity = highest_vulnerability_severity(&sections[11].items);
    }

    let total_findings = sections.iter().map(DashboardSection::count).sum();
    let highest_severity = sections
        .iter()
        .filter(|section| !section.items.is_empty())
        .map(|section| section.severity)
        .min()
        .unwrap_or(DashboardSeverity::Info);

    Ok(DashboardData {
        path: path.display().to_string(),
        total_files,
        total_findings,
        highest_severity,
        duration_ms: start.elapsed().as_millis() as u64,
        vuln_db_version: vuln_db.version.clone(),
        sections,
    })
}

fn highest_vulnerability_severity(items: &[String]) -> DashboardSeverity {
    items
        .iter()
        .filter_map(|item| {
            if item.contains("(CRITICAL)") {
                Some(DashboardSeverity::Critical)
            } else if item.contains("(HIGH)") {
                Some(DashboardSeverity::High)
            } else if item.contains("(MEDIUM)") {
                Some(DashboardSeverity::Medium)
            } else if item.contains("(LOW)") {
                Some(DashboardSeverity::Low)
            } else {
                None
            }
        })
        .min()
        .unwrap_or(DashboardSeverity::High)
}

fn format_vuln_match(matched: VulnMatch) -> String {
    format!(
        "{} ({}) at {}:{}",
        matched.name,
        matched.severity.to_uppercase(),
        matched.file,
        matched.line
    )
}

fn render_snapshot(data: &DashboardData) -> String {
    let mut lines = vec![
        "Sanctifier TUI Snapshot".to_string(),
        format!("Project: {}", data.path),
        format!("Files scanned: {}", data.total_files),
        format!("Total findings: {}", data.total_findings),
        format!("Highest severity: {}", data.highest_severity.label()),
        format!("Duration: {} ms", data.duration_ms),
        format!("Vuln DB: {}", data.vuln_db_version),
        String::new(),
    ];

    for section in &data.sections {
        lines.push(format!(
            "{} [{}] {} finding(s)",
            section.title,
            section.code,
            section.count()
        ));
        if section.items.is_empty() {
            lines.push("  - No findings detected.".to_string());
        } else {
            for item in &section.items {
                lines.push(format!("  - {}", item));
            }
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

fn run_terminal_dashboard(data: DashboardData) -> anyhow::Result<()> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to create terminal backend")?;
    let mut app = DashboardApp::new(data);

    let result = run_event_loop(&mut terminal, &mut app);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut DashboardApp,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| render_dashboard(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => app.next_tab(),
                KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => app.previous_tab(),
                KeyCode::Down | KeyCode::Char('j') => app.next_item(),
                KeyCode::Up | KeyCode::Char('k') => app.previous_item(),
                _ => {}
            }
        }
    }
}

fn render_dashboard(frame: &mut Frame<'_>, app: &DashboardApp) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(7),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_header(frame, root[0], app);
    render_summary(frame, root[1], app);
    render_tabs(frame, root[2], app);
    render_body(frame, root[3], app);
    render_footer(frame, root[4]);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &DashboardApp) {
    let status = format!(
        "{} severity | {} finding(s) | {} file(s)",
        app.data.highest_severity.label(),
        app.data.total_findings,
        app.data.total_files
    );
    let text = Text::from(vec![
        Line::from(vec![
            Span::styled(
                "Sanctifier TUI",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                status,
                Style::default().fg(app.data.highest_severity.color()),
            ),
        ]),
        Line::from(app.data.path.clone()),
    ]);
    let widget =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Overview"));
    frame.render_widget(widget, area);
}

fn render_summary(frame: &mut Frame<'_>, area: Rect, app: &DashboardApp) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let cards = [
        (
            "Critical",
            count_by_severity(&app.data, DashboardSeverity::Critical),
        ),
        (
            "High",
            count_by_severity(&app.data, DashboardSeverity::High),
        ),
        (
            "Medium",
            count_by_severity(&app.data, DashboardSeverity::Medium),
        ),
        ("Low/Info", count_by_low_info(&app.data)),
    ];

    for (idx, (label, count)) in cards.iter().enumerate() {
        let severity = match *label {
            "Critical" => DashboardSeverity::Critical,
            "High" => DashboardSeverity::High,
            "Medium" => DashboardSeverity::Medium,
            _ => DashboardSeverity::Low,
        };
        let card = Paragraph::new(format!("{}\n{}", count, label))
            .style(
                Style::default()
                    .fg(severity.color())
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(card, chunks[idx]);
    }
}

fn render_tabs(frame: &mut Frame<'_>, area: Rect, app: &DashboardApp) {
    let titles: Vec<Line<'_>> = app
        .data
        .sections
        .iter()
        .map(|section| Line::from(format!("{} ({})", section.title, section.count())))
        .collect();

    let tabs = Tabs::new(titles)
        .select(app.active_tab)
        .block(Block::default().borders(Borders::ALL).title("Categories"))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(app.current_section().severity.color())
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}

fn render_body(frame: &mut Frame<'_>, area: Rect, app: &DashboardApp) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    let section = app.current_section();
    let items: Vec<ListItem<'_>> = if section.items.is_empty() {
        vec![ListItem::new("No findings in this category.")]
    } else {
        section
            .items
            .iter()
            .map(|item| ListItem::new(item.as_str()))
            .collect()
    };

    let mut state = ListState::default();
    if !section.items.is_empty() {
        state.select(Some(app.selected_item));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("{} [{}]", section.title, section.code)),
        )
        .highlight_style(
            Style::default()
                .bg(section.severity.color())
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, chunks[0], &mut state);

    let detail_text = selected_detail(section, app.selected_item, &app.data);
    let detail = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .wrap(Wrap { trim: false });
    frame.render_widget(Clear, chunks[1]);
    frame.render_widget(detail, chunks[1]);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect) {
    let footer = Paragraph::new("Tab/Left/Right switch categories  Up/Down move  q exits")
        .block(Block::default().borders(Borders::ALL).title("Keys"));
    frame.render_widget(footer, area);
}

fn selected_detail(
    section: &DashboardSection,
    selected_item: usize,
    data: &DashboardData,
) -> Text<'static> {
    if section.items.is_empty() {
        return Text::from(vec![
            Line::from("No findings detected in this category."),
            Line::from(format!("Database version: {}", data.vuln_db_version)),
            Line::from(format!("Render time: {} ms", data.duration_ms)),
        ]);
    }

    let detail = &section.items[selected_item.min(section.items.len() - 1)];
    Text::from(vec![
        Line::from(vec![
            Span::styled(
                section.severity.label(),
                Style::default()
                    .fg(section.severity.color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("  {} [{}]", section.title, section.code)),
        ]),
        Line::from(""),
        Line::from(detail.clone()),
        Line::from(""),
        Line::from(format!("Project: {}", data.path)),
        Line::from(format!("Files scanned: {}", data.total_files)),
    ])
}

fn count_by_severity(data: &DashboardData, severity: DashboardSeverity) -> usize {
    data.sections
        .iter()
        .filter(|section| section.severity == severity)
        .map(DashboardSection::count)
        .sum()
}

fn count_by_low_info(data: &DashboardData) -> usize {
    data.sections
        .iter()
        .filter(|section| {
            section.severity == DashboardSeverity::Low
                || section.severity == DashboardSeverity::Info
        })
        .map(DashboardSection::count)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_includes_empty_sections() {
        let data = DashboardData {
            path: "demo.rs".into(),
            total_files: 1,
            total_findings: 0,
            highest_severity: DashboardSeverity::Info,
            duration_ms: 14,
            vuln_db_version: "test".into(),
            sections: vec![DashboardSection {
                title: "Authentication Gaps".into(),
                code: "S001".into(),
                severity: DashboardSeverity::Critical,
                items: Vec::new(),
            }],
        };

        let rendered = render_snapshot(&data);
        assert!(rendered.contains("Sanctifier TUI Snapshot"));
        assert!(rendered.contains("No findings detected."));
    }

    #[test]
    fn vuln_match_severity_prefers_critical() {
        let items = vec![
            "Issue One (MEDIUM) at file.rs:10".to_string(),
            "Issue Two (CRITICAL) at file.rs:20".to_string(),
        ];

        assert_eq!(
            highest_vulnerability_severity(&items),
            DashboardSeverity::Critical
        );
    }
}
