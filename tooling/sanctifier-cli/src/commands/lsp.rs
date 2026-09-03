use clap::Args;
use sanctifier_core::{Analyzer, SanctifyConfig};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use std::sync::{Arc, Mutex};

#[derive(Args, Debug)]
pub struct LspArgs {
    /// Enable debug logging to stderr
    #[arg(long)]
    debug: bool,
}

pub fn exec(args: LspArgs) -> anyhow::Result<()> {
    let server = SanctifierLanguageServer::new(args.debug);
    server.run()
}

struct SanctifierLanguageServer {
    debug: bool,
    documents: Arc<Mutex<HashMap<String, String>>>,
}

impl SanctifierLanguageServer {
    fn new(debug: bool) -> Self {
        Self {
            debug,
            documents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn log(&self, msg: &str) {
        if self.debug {
            eprintln!("[sanctifier-lsp] {}", msg);
        }
    }

    fn run(&self) -> anyhow::Result<()> {
        self.log("LSP server starting");

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        let mut reader = stdin.lock();
        let mut line_buffer = String::new();

        // Main server loop
        loop {
            line_buffer.clear();
            if reader.read_line(&mut line_buffer)? == 0 {
                break;
            }

            if line_buffer.is_empty() || line_buffer.starts_with('\n') {
                continue;
            }

            if line_buffer.starts_with("Content-Length:") {
                let content_length = line_buffer
                    .strip_prefix("Content-Length:")
                    .unwrap_or("")
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(0);

                // Read the empty line
                line_buffer.clear();
                let _ = reader.read_line(&mut line_buffer);

                // Read the message
                let mut message = vec![0; content_length];
                reader.read_exact(&mut message)?;
                let message_str = String::from_utf8(message)?;

                self.log(&format!(
                    "Received message: {}",
                    message_str.chars().take(100).collect::<String>()
                ));

                match serde_json::from_str::<Value>(&message_str) {
                    Ok(msg) => {
                        if let Some(result) = self.handle_message(&msg) {
                            let response = result.to_string();
                            let response_bytes = response.as_bytes();
                            write!(
                                stdout,
                                "Content-Length: {}\r\n\r\n{}",
                                response_bytes.len(),
                                response
                            )?;
                            stdout.flush()?;
                            self.log(&format!(
                                "Sent response: {}",
                                response.chars().take(100).collect::<String>()
                            ));
                        }
                    }
                    Err(e) => {
                        self.log(&format!("Failed to parse message: {}", e));
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_message(&self, msg: &Value) -> Option<Value> {
        let id = msg.get("id")?;

        match msg.get("method").and_then(|m| m.as_str()) {
            Some("initialize") => {
                self.log("Handling initialize request");
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "capabilities": {
                            "textDocumentSync": 1,
                            "diagnosticProvider": {
                                "interFileDependencies": false,
                                "workspaceDiagnostics": false
                            },
                            "codeActionProvider": true,
                            "hoverProvider": true,
                        }
                    }
                }))
            }
            Some("shutdown") => {
                self.log("Handling shutdown request");
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": null
                }))
            }
            Some("textDocument/codeAction") => {
                self.log("Handling codeAction request");
                let params = msg.get("params")?;
                let uri = params
                    .get("textDocument")
                    .and_then(|t| t.get("uri"))
                    .and_then(|u| u.as_str())?;

                let docs = self.documents.lock().unwrap();
                if let Some(text) = docs.get(uri) {
                    let actions = self.get_code_actions(text);
                    return Some(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": actions
                    }));
                }

                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": []
                }))
            }
            Some("initialized") | Some("exit") => {
                self.log(&format!(
                    "Ignoring notification: {}",
                    msg.get("method").and_then(|m| m.as_str()).unwrap_or("?")
                ));
                None
            }
            _ => {
                self.log(&format!("Unknown method: {:?}", msg.get("method")));
                None
            }
        }
    }

    #[allow(dead_code)]
    fn handle_notification(&self, notif: &Value) -> anyhow::Result<Option<Vec<Value>>> {
        let method = notif.get("method").and_then(|m| m.as_str()).unwrap_or("");
        self.log(&format!("Handling notification: {}", method));

        match method {
            "textDocument/didOpen" => {
                let params = notif
                    .get("params")
                    .ok_or_else(|| anyhow::anyhow!("Missing params"))?;
                let uri = params
                    .get("textDocument")
                    .and_then(|t| t.get("uri"))
                    .and_then(|u| u.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing uri"))?;
                let text = params
                    .get("textDocument")
                    .and_then(|t| t.get("text"))
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing text"))?;

                let mut docs = self.documents.lock().unwrap();
                docs.insert(uri.to_string(), text.to_string());

                let diagnostics = self.analyze_document(text);
                Ok(Some(diagnostics))
            }
            "textDocument/didChange" => {
                let params = notif
                    .get("params")
                    .ok_or_else(|| anyhow::anyhow!("Missing params"))?;
                let uri = params
                    .get("textDocument")
                    .and_then(|t| t.get("uri"))
                    .and_then(|u| u.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing uri"))?;
                let text = params
                    .get("contentChanges")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing text"))?;

                let mut docs = self.documents.lock().unwrap();
                docs.insert(uri.to_string(), text.to_string());

                let diagnostics = self.analyze_document(text);
                Ok(Some(diagnostics))
            }
            "textDocument/didClose" => {
                let params = notif
                    .get("params")
                    .ok_or_else(|| anyhow::anyhow!("Missing params"))?;
                let uri = params
                    .get("textDocument")
                    .and_then(|t| t.get("uri"))
                    .and_then(|u| u.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing uri"))?;

                let mut docs = self.documents.lock().unwrap();
                docs.remove(uri);
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    #[allow(dead_code)]
    fn analyze_document(&self, text: &str) -> Vec<Value> {
        let config = SanctifyConfig::default();
        let analyzer = Analyzer::new(config);
        let mut diagnostics = Vec::new();

        // Scan auth gaps
        let auth_gaps = analyzer.scan_auth_gaps(text);
        for gap in auth_gaps {
            if let Some(line_num) = self.find_function_line(&gap, text) {
                diagnostics.push(json!({
                    "range": {
                        "start": { "line": line_num, "character": 0 },
                        "end": { "line": line_num, "character": 100 }
                    },
                    "severity": 2,
                    "code": "S001",
                    "source": "sanctifier",
                    "message": format!(
                        "Function '{}' modifies state without authorization check. Add require_auth or require_auth_for_args.",
                        gap
                    ),
                }));
            }
        }

        // Scan panics
        let panics = analyzer.scan_panics(text);
        for panic in panics {
            if let Some(line_num) = self.find_function_line(&panic.function_name, text) {
                diagnostics.push(json!({
                    "range": {
                        "start": { "line": line_num, "character": 0 },
                        "end": { "line": line_num, "character": 100 }
                    },
                    "severity": 2,
                    "code": "S002",
                    "source": "sanctifier",
                    "message": format!(
                        "Found {} in function '{}'. Consider using Result or Option instead.",
                        panic.issue_type, panic.function_name
                    ),
                }));
            }
        }

        // Scan arithmetic overflow
        let arithmetic_issues = analyzer.scan_arithmetic_overflow(text);
        for issue in arithmetic_issues {
            if let Some(line_num) = self.find_function_line(&issue.function_name, text) {
                diagnostics.push(json!({
                    "range": {
                        "start": { "line": line_num, "character": 0 },
                        "end": { "line": line_num, "character": 100 }
                    },
                    "severity": 2,
                    "code": "S003",
                    "source": "sanctifier",
                    "message": format!(
                        "Unchecked arithmetic operation '{}' in function '{}'. {}",
                        issue.operation, issue.function_name, issue.suggestion
                    ),
                }));
            }
        }

        // Scan unsafe patterns
        let unsafe_patterns = analyzer.analyze_unsafe_patterns(text);
        for pattern in unsafe_patterns {
            let severity = match pattern.pattern_type {
                sanctifier_core::PatternType::Panic => 1,
                sanctifier_core::PatternType::Unwrap => 2,
                sanctifier_core::PatternType::Expect => 2,
            };
            diagnostics.push(json!({
                "range": {
                    "start": { "line": pattern.line as i32 - 1, "character": 0 },
                    "end": { "line": pattern.line as i32 - 1, "character": 100 }
                },
                "severity": severity,
                "code": "S006",
                "source": "sanctifier",
                "message": format!("Unsafe pattern: {}. Consider safer alternatives.", pattern.snippet),
            }));
        }

        // Analyze ledger size
        let size_warnings = analyzer.analyze_ledger_size(text);
        for warning in size_warnings {
            if let Some(line_num) = self.find_struct_line(&warning.struct_name, text) {
                let (severity, message) = match warning.level {
                    sanctifier_core::SizeWarningLevel::ExceedsLimit => (
                        1,
                        format!(
                            "Struct '{}' exceeds ledger entry limit ({} > {} bytes)",
                            warning.struct_name, warning.estimated_size, warning.limit
                        ),
                    ),
                    sanctifier_core::SizeWarningLevel::ApproachingLimit => (
                        2,
                        format!(
                            "Struct '{}' approaching ledger entry limit ({} / {} bytes)",
                            warning.struct_name, warning.estimated_size, warning.limit
                        ),
                    ),
                };
                diagnostics.push(json!({
                    "range": {
                        "start": { "line": line_num, "character": 0 },
                        "end": { "line": line_num, "character": 100 }
                    },
                    "severity": severity,
                    "code": "S004",
                    "source": "sanctifier",
                    "message": message,
                }));
            }
        }

        // Analyze custom rules
        let custom_matches = analyzer.analyze_custom_rules(text);
        for custom_match in custom_matches {
            diagnostics.push(json!({
                "range": {
                    "start": { "line": custom_match.line as i32 - 1, "character": 0 },
                    "end": { "line": custom_match.line as i32 - 1, "character": 100 }
                },
                "severity": 3,
                "code": "S007",
                "source": "sanctifier",
                "message": format!("Custom rule '{}': {}", custom_match.rule_name, custom_match.snippet),
            }));
        }

        diagnostics
    }

    fn get_code_actions(&self, text: &str) -> Vec<Value> {
        let config = SanctifyConfig::default();
        let analyzer = Analyzer::new(config);
        let mut actions = Vec::new();

        // Auth gap code actions
        let auth_gaps = analyzer.scan_auth_gaps(text);
        for gap in auth_gaps {
            actions.push(json!({
                "title": format!("Add require_auth to function '{}'", gap),
                "kind": "quickfix",
                "isPreferred": true,
            }));
        }

        // Arithmetic overflow code actions
        let arithmetic_issues = analyzer.scan_arithmetic_overflow(text);
        for issue in arithmetic_issues {
            actions.push(json!({
                "title": format!("Use checked_{} instead of '{}'", issue.operation.trim_end_matches('='), issue.operation),
                "kind": "quickfix",
                "isPreferred": true,
            }));
        }

        actions
    }

    #[allow(dead_code)]
    fn find_function_line(&self, fn_name: &str, text: &str) -> Option<usize> {
        let pattern = format!(r"pub\s+fn\s+{}\s*\(", fn_name);
        let re = regex::Regex::new(&pattern).ok()?;
        for (line_num, line) in text.lines().enumerate() {
            if re.is_match(line) {
                return Some(line_num);
            }
        }
        None
    }

    #[allow(dead_code)]
    fn find_struct_line(&self, struct_name: &str, text: &str) -> Option<usize> {
        let pattern = format!(r"(?:pub\s+)?struct\s+{}\s*(?:\{{|<)", struct_name);
        let re = regex::Regex::new(&pattern).ok()?;
        for (line_num, line) in text.lines().enumerate() {
            if re.is_match(line) {
                return Some(line_num);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_analyze_auth_gap() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn set_data(env: Env, val: u32) {
                    env.storage().instance().set(&DataKey::Val, &val);
                }
            }
        "#;

        let gaps = analyzer.scan_auth_gaps(source);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0], "set_data");
    }

    #[test]
    fn test_lsp_analyze_arithmetic() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn add(env: Env, a: u64, b: u64) -> u64 {
                    a + b
                }
            }
        "#;

        let issues = analyzer.scan_arithmetic_overflow(source);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].operation, "+");
    }

    #[test]
    fn test_lsp_analyze_panic() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn dangerous(env: Env) {
                    panic!("error");
                }
            }
        "#;

        let issues = analyzer.scan_panics(source);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "panic!");
    }

    #[test]
    fn test_lsp_analyze_unsafe_patterns() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn risky(env: Env) {
                    let x = Some(5);
                    x.unwrap();
                }
            }
        "#;

        let patterns = analyzer.analyze_unsafe_patterns(source);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_lsp_analyze_ledger_size() {
        let config = SanctifyConfig {
            ledger_limit: 50,
            ..Default::default()
        };
        let analyzer = Analyzer::new(config);
        let source = r#"
            #[contracttype]
            pub struct ExceedsLimit {
                pub buffer: Bytes,
            }
        "#;

        let warnings = analyzer.analyze_ledger_size(source);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_lsp_multiple_issues() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                    // Missing auth
                    let from_balance: i128 = env.storage().persistent().get(&from).unwrap_or(0);
                    // Unchecked arithmetic
                    let new_balance = from_balance - amount;
                    env.storage().persistent().set(&from, &new_balance);
                }
                
                pub fn mint(env: Env, to: Address, amount: i128) {
                    // Missing auth, unchecked arithmetic
                    let current = env.storage().persistent().get(&to).unwrap_or(0);
                    let new_balance = current + amount;
                    env.storage().persistent().set(&to, &new_balance);
                }
            }
        "#;

        let gaps = analyzer.scan_auth_gaps(source);
        assert_eq!(gaps.len(), 2);

        let arithmetic = analyzer.scan_arithmetic_overflow(source);
        assert!(arithmetic.len() >= 2);
    }

    #[test]
    fn test_lsp_no_false_positives_for_safe_code() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl SafeContract {
                pub fn safe_transfer(env: Env, from: Address, to: Address, amount: i128) {
                    from.require_auth();
                    let from_balance: i128 = env.storage().persistent().get(&from).unwrap_or(0);
                    let new_balance = from_balance.checked_sub(amount).expect("underflow");
                    env.storage().persistent().set(&from, &new_balance);
                }
            }
        "#;

        let gaps = analyzer.scan_auth_gaps(source);
        assert_eq!(gaps.len(), 0);

        let arithmetic = analyzer.scan_arithmetic_overflow(source);
        assert_eq!(arithmetic.len(), 0);
    }
}
