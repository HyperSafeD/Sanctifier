//! Circom `.circom` source-file parser for sanctifier-core (#1227).
//!
//! Produces a lightweight intermediate representation sufficient to power
//! Z007 (under-constrained signals) and Z008 (unchecked component outputs).
//!
//! ## Coverage
//! This parser covers the subset of Circom needed for ZK security analysis:
//! - Template declarations and their parameter lists
//! - Signal declarations (`signal input`, `signal output`, `signal`)
//! - Constraint expressions (`===`, `<==`, `==>`)
//! - Component instantiations (`component c = Template(...)`)
//! - Known gaps: `pragma` versions, `include` directives, and `function`
//!   blocks are parsed but not deeply analysed.

/// A parsed circom template.
#[derive(Debug, Clone, PartialEq)]
pub struct CircomTemplate {
    /// Template name (e.g. `"Multiplier"`).
    pub name: String,
    /// Template-level parameters (not signals).
    pub params: Vec<String>,
    /// Signals declared inside this template.
    pub signals: Vec<Signal>,
    /// Component instantiations inside this template.
    pub components: Vec<ComponentInst>,
    /// Constraint expressions found in this template.
    pub constraints: Vec<String>,
}

/// A signal declaration inside a template.
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    /// Signal name.
    pub name: String,
    /// Direction: `Input`, `Output`, or `Intermediate`.
    pub direction: SignalDirection,
    /// Whether this signal appears in at least one constraint.
    pub is_constrained: bool,
}

/// Signal direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalDirection {
    Input,
    Output,
    Intermediate,
}

/// A component instantiation.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentInst {
    /// Variable name of the component instance.
    pub var_name: String,
    /// Template being instantiated.
    pub template_name: String,
}

/// The top-level result of parsing a `.circom` file.
#[derive(Debug, Clone, Default)]
pub struct CircomFile {
    /// All templates defined in the file.
    pub templates: Vec<CircomTemplate>,
    /// Pragma version string if present.
    pub pragma_version: Option<String>,
    /// Include paths.
    pub includes: Vec<String>,
}

/// Parse errors.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedToken { line: usize, token: String },
    UnterminatedTemplate { name: String },
    UnknownSignalDirection(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken { line, token } => {
                write!(f, "unexpected token `{}` at line {}", token, line)
            }
            ParseError::UnterminatedTemplate { name } => {
                write!(f, "unterminated template `{}`", name)
            }
            ParseError::UnknownSignalDirection(d) => {
                write!(f, "unknown signal direction `{}`", d)
            }
        }
    }
}

/// Parse a circom source string into a [`CircomFile`].
pub fn parse(source: &str) -> Result<CircomFile, ParseError> {
    let mut file = CircomFile::default();
    let mut lines = source.lines().enumerate().peekable();

    while let Some((line_no, raw_line)) = lines.next() {
        let line = raw_line.trim();

        // Pragma
        if line.starts_with("pragma circom") {
            let ver = line
                .trim_start_matches("pragma circom")
                .trim()
                .trim_end_matches(';')
                .to_string();
            file.pragma_version = Some(ver);
            continue;
        }

        // Include
        if line.starts_with("include ") {
            let path = line
                .trim_start_matches("include ")
                .trim()
                .trim_matches('"')
                .trim_end_matches(';')
                .to_string();
            file.includes.push(path);
            continue;
        }

        // Template declaration
        if line.starts_with("template ") {
            let template = parse_template(line_no + 1, line, &mut lines)?;
            file.templates.push(template);
            continue;
        }
    }

    Ok(file)
}

/// Parse one template block, consuming lines up to the matching `}`.
fn parse_template<'a>(
    _start_line: usize,
    header: &str,
    lines: &mut std::iter::Peekable<impl Iterator<Item = (usize, &'a str)>>,
) -> Result<CircomTemplate, ParseError> {
    // Parse template name and params from the header line, e.g.
    // "template Multiplier(n) {"
    let header = header.trim();
    let after_kw = header.trim_start_matches("template").trim();
    let (name, params) = if let Some(paren) = after_kw.find('(') {
        let name = after_kw[..paren].trim().to_string();
        let close = after_kw.find(')').unwrap_or(after_kw.len());
        let params_str = &after_kw[paren + 1..close];
        let params: Vec<String> = if params_str.trim().is_empty() {
            vec![]
        } else {
            params_str
                .split(',')
                .map(|p| p.trim().to_string())
                .collect()
        };
        (name, params)
    } else {
        let name = after_kw.trim_end_matches('{').trim().to_string();
        (name, vec![])
    };

    let mut signals: Vec<Signal> = Vec::new();
    let mut components: Vec<ComponentInst> = Vec::new();
    let mut raw_constraints: Vec<String> = Vec::new();
    let mut depth = if header.contains('{') { 1usize } else { 0 };

    // Collect constraint expressions so we can mark signals as constrained.
    let mut constraint_body = String::new();

    for (_ln, raw) in lines.by_ref() {
        // Strip line comments before anything else: a comment must not affect
        // brace depth, and — critically — must not be mistaken for a constraint
        // mentioning a signal (`// inter is never constrained`).
        let line = raw.split("//").next().unwrap_or(raw).trim();

        // Track brace depth to know when the template closes.
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    if depth == 0 {
                        return Err(ParseError::UnterminatedTemplate { name });
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }

        if depth == 0 {
            break;
        }

        // Signal declaration
        if line.starts_with("signal") {
            if let Some(sig) = parse_signal_decl(line) {
                signals.push(sig);
            }
            continue;
        }

        // Component instantiation
        if line.starts_with("component ") {
            if let Some(comp) = parse_component_decl(line) {
                components.push(comp);
            }
            continue;
        }

        // Constraint operator
        if line.contains("===") || line.contains("<==") || line.contains("==>") {
            raw_constraints.push(line.to_string());
        }

        // Only executable lines feed the constrained-signal scan; declarations
        // and comments were skipped above.
        constraint_body.push(' ');
        constraint_body.push_str(line);
    }

    // Mark signals that appear in a constraint.
    for signal in signals.iter_mut() {
        if raw_constraints.iter().any(|c| c.contains(&signal.name))
            || constraint_body.contains(&format!(" {} ", signal.name))
        {
            signal.is_constrained = true;
        }
    }

    Ok(CircomTemplate {
        name,
        params,
        signals,
        components,
        constraints: raw_constraints,
    })
}

fn parse_signal_decl(line: &str) -> Option<Signal> {
    // Forms:
    //   signal input in1;
    //   signal output out;
    //   signal inter;
    let rest = line.trim_start_matches("signal").trim();
    let (direction, rest) = if rest.starts_with("input ") {
        (
            SignalDirection::Input,
            rest.trim_start_matches("input").trim(),
        )
    } else if rest.starts_with("output ") {
        (
            SignalDirection::Output,
            rest.trim_start_matches("output").trim(),
        )
    } else {
        (SignalDirection::Intermediate, rest)
    };

    let name = rest
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()?
        .trim()
        .to_string();

    if name.is_empty() {
        return None;
    }

    Some(Signal {
        name,
        direction,
        is_constrained: false,
    })
}

fn parse_component_decl(line: &str) -> Option<ComponentInst> {
    // "component c = Template(args);"
    let rest = line.trim_start_matches("component").trim();
    let eq_pos = rest.find('=')?;
    let var_name = rest[..eq_pos].trim().to_string();
    let rhs = rest[eq_pos + 1..].trim();
    let paren = rhs.find('(')?;
    let template_name = rhs[..paren].trim().to_string();
    Some(ComponentInst {
        var_name,
        template_name,
    })
}

/// Return all signals that are never referenced in a constraint expression.
pub fn unconstrained_signals(template: &CircomTemplate) -> Vec<&Signal> {
    template
        .signals
        .iter()
        .filter(|s| !s.is_constrained)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MULTIPLIER: &str = r#"
pragma circom 2.0.0;

template Multiplier(n) {
    signal input a;
    signal input b;
    signal output c;

    c <== a * b;
}
"#;

    const UNDER_CONSTRAINED: &str = r#"
pragma circom 2.0.0;

template LeakyMult() {
    signal input a;
    signal input b;
    signal output c;
    signal inter;

    c <== a * b;
    // inter is never constrained
}
"#;

    #[test]
    fn parses_pragma_version() {
        let f = parse(MULTIPLIER).unwrap();
        assert_eq!(f.pragma_version.as_deref(), Some("2.0.0"));
    }

    #[test]
    fn parses_template_name_and_params() {
        let f = parse(MULTIPLIER).unwrap();
        assert_eq!(f.templates.len(), 1);
        assert_eq!(f.templates[0].name, "Multiplier");
        assert_eq!(f.templates[0].params, vec!["n"]);
    }

    #[test]
    fn parses_signal_declarations() {
        let f = parse(MULTIPLIER).unwrap();
        let t = &f.templates[0];
        assert_eq!(t.signals.len(), 3);

        let a = t.signals.iter().find(|s| s.name == "a").unwrap();
        assert_eq!(a.direction, SignalDirection::Input);

        let c = t.signals.iter().find(|s| s.name == "c").unwrap();
        assert_eq!(c.direction, SignalDirection::Output);
    }

    #[test]
    fn constrained_signals_marked_correctly() {
        let f = parse(MULTIPLIER).unwrap();
        let t = &f.templates[0];
        for sig in &t.signals {
            assert!(
                sig.is_constrained,
                "signal {} must be constrained",
                sig.name
            );
        }
    }

    #[test]
    fn detects_unconstrained_intermediate_signal() {
        let f = parse(UNDER_CONSTRAINED).unwrap();
        let t = &f.templates[0];
        let unconstrained = unconstrained_signals(t);
        assert!(
            unconstrained.iter().any(|s| s.name == "inter"),
            "inter must be detected as unconstrained"
        );
    }

    #[test]
    fn constrained_signals_not_flagged() {
        let f = parse(MULTIPLIER).unwrap();
        let unconstrained = unconstrained_signals(&f.templates[0]);
        assert!(
            unconstrained.is_empty(),
            "well-constrained circuit must have no unconstrained signals"
        );
    }

    #[test]
    fn empty_source_parses_successfully() {
        let f = parse("").unwrap();
        assert!(f.templates.is_empty());
    }

    #[test]
    fn parses_template_instantiation_constraint() {
        let source = r#"
pragma circom 2.0.0;

template IsZero() {
    signal input in;
    signal output out;
    signal inv;

    inv <-- in != 0 ? 1/in : 0;
    out <== -in * inv + 1;
    in * out === 0;
}
"#;
        let f = parse(source).unwrap();
        assert_eq!(f.templates.len(), 1);
        let t = &f.templates[0];
        assert_eq!(t.name, "IsZero");
        assert!(!t.constraints.is_empty(), "constraints must be parsed");
    }
}
