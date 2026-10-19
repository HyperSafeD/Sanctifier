//! Event analysis pass.

use crate::{EventIssue, EventIssueType};
use std::collections::{HashMap, HashSet};

fn extract_topics(line: &str) -> String {
    if let Some(start_paren) = line.find('(') {
        let after_publish = &line[start_paren + 1..];
        if let Some(end_paren) = after_publish.rfind(')') {
            let topics_content = &after_publish[..end_paren];
            if topics_content.contains(',') || topics_content.starts_with('(') {
                return topics_content.to_string();
            }
        }
    }
    if let Some(vec_start) = line.find("vec![") {
        let after_vec = &line[vec_start + 5..];
        if let Some(end_bracket) = after_vec.find(']') {
            return after_vec[..end_bracket].to_string();
        }
    }
    String::new()
}

fn extract_event_name(line: &str) -> Option<String> {
    if let Some(start) = line.find('(') {
        let content = &line[start..];
        if let Some(name_end) = content.find(',') {
            let name_part = &content[1..name_end];
            let clean_name = name_part.trim().trim_matches('"');
            if !clean_name.is_empty() {
                return Some(clean_name.to_string());
            }
        } else if let Some(end_paren) = content.find(')') {
            let name_part = &content[1..end_paren];
            let clean_name = name_part.trim().trim_matches('"');
            if !clean_name.is_empty() {
                return Some(clean_name.to_string());
            }
        }
    }
    None
}

/// Scans for `env.events().publish(topics, data)` and checks:
/// 1. Consistency of topic counts for the same event name.
/// 2. Opportunities to use `symbol_short!` for gas savings.
pub fn scan_events(source: &str) -> Vec<EventIssue> {
    let mut issues = Vec::new();
    let mut event_schemas: HashMap<String, Vec<usize>> = HashMap::new();
    let mut issue_locations: HashSet<String> = HashSet::new();

    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();

        if line.contains("env.events().publish(") || line.contains("env.events().emit(") {
            let topics_str = extract_topics(line);
            let topic_count = if topics_str.is_empty() {
                0
            } else {
                topics_str.matches(',').count() + 1
            };

            let event_name =
                extract_event_name(line).unwrap_or_else(|| format!("unknown_{}", line_num));

            let location = format!("line {}", line_num + 1);

            if let Some(previous_counts) = event_schemas.get(&event_name) {
                for &prev_count in previous_counts {
                    if prev_count != topic_count {
                        let issue_key = format!("{}:{}:inconsistent", event_name, line_num + 1);
                        if !issue_locations.contains(&issue_key) {
                            issue_locations.insert(issue_key);
                            issues.push(EventIssue {
                                function_name: "unknown".to_string(),
                                event_name: event_name.clone(),
                                issue_type: EventIssueType::InconsistentSchema,
                                message: format!(
                                    "Event '{}' has inconsistent topic count. Previous: {}, Current: {}",
                                    event_name, prev_count, topic_count
                                ),
                                location: location.clone(),
                            });
                        }
                    }
                }
            }

            event_schemas
                .entry(event_name.clone())
                .or_default()
                .push(topic_count);

            if !line.contains("symbol_short!") && topic_count > 0 {
                let has_string_topic = line.contains('"') || line.contains("String");
                if has_string_topic {
                    let issue_key = format!("{}:{}:gas_optimization", event_name, line_num + 1);
                    if !issue_locations.contains(&issue_key) {
                        issue_locations.insert(issue_key);
                        issues.push(EventIssue {
                            function_name: "unknown".to_string(),
                            event_name,
                            issue_type: EventIssueType::OptimizableTopic,
                            message:
                                "Consider using symbol_short! for short topic names to save gas."
                                    .to_string(),
                            location: format!("line {}", line_num + 1),
                        });
                    }
                }
            }
        }
    }

    issues
}
