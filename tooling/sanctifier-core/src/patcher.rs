//! Source-level auto-patching.

use crate::rules::Patch;

/// Applies [`Patch`]es to source text.
pub struct Patcher;

impl Patcher {
    /// Apply a set of non-overlapping patches to `source`.
    pub fn apply_patches(source: &str, patches: &[Patch]) -> String {
        if patches.is_empty() {
            return source.to_string();
        }

        let mut sorted_patches = patches.to_vec();
        // Sort by position in reverse order (bottom-up, right-to-left)
        // to avoid shifting offsets of subsequent patches.
        sorted_patches.sort_by(|a, b| {
            if a.start_line != b.start_line {
                b.start_line.cmp(&a.start_line)
            } else {
                b.start_column.cmp(&a.start_column)
            }
        });

        let mut result = source.to_string();

        for patch in sorted_patches {
            let (start_offset, end_offset) = match Self::calculate_offsets(&result, &patch) {
                Some(offsets) => offsets,
                None => continue,
            };

            result.replace_range(start_offset..end_offset, &patch.replacement);
        }

        result
    }

    fn calculate_offsets(source: &str, patch: &Patch) -> Option<(usize, usize)> {
        let mut start_offset = None;
        let mut end_offset = None;
        let mut current_offset = 0;
        let mut current_line = 1;

        // Simple but potentially slow off-set calculator.
        // For CLI use, it should be fine.
        for (i, c) in source.char_indices() {
            if current_line == patch.start_line && start_offset.is_none() {
                // syn columns are Unicode-aware char offsets on that line
                // But we need to be careful about tab widths etc.
                // For now assuming 1 char = 1 column.
                let line_text = &source[current_offset..];
                for (col_counter, (j, c2)) in line_text.char_indices().enumerate() {
                    if col_counter == patch.start_column {
                        start_offset = Some(current_offset + j);
                        break;
                    }
                    if c2 == '\n' {
                        break;
                    }
                }
            }

            if current_line == patch.end_line && end_offset.is_none() {
                let line_text = &source[current_offset..];
                for (col_counter, (j, c2)) in line_text.char_indices().enumerate() {
                    if col_counter == patch.end_column {
                        end_offset = Some(current_offset + j);
                        break;
                    }
                    if c2 == '\n' {
                        break;
                    }
                }
            }

            if c == '\n' {
                current_line += 1;
                current_offset = i + 1;
            }
        }

        // Handle end of file cases
        if start_offset.is_none() && patch.start_line == current_line {
            // check if column is at the very end
            let line_text = &source[current_offset..];
            if patch.start_column <= line_text.chars().count() {
                start_offset = Some(
                    current_offset
                        + line_text
                            .char_indices()
                            .nth(patch.start_column)
                            .map(|(i, _)| i)
                            .unwrap_or(line_text.len()),
                );
            }
        }
        if end_offset.is_none() && patch.end_line == current_line {
            let line_text = &source[current_offset..];
            if patch.end_column <= line_text.chars().count() {
                end_offset = Some(
                    current_offset
                        + line_text
                            .char_indices()
                            .nth(patch.end_column)
                            .map(|(i, _)| i)
                            .unwrap_or(line_text.len()),
                );
            }
        }

        match (start_offset, end_offset) {
            (Some(s), Some(e)) => Some((s, e)),
            _ => None,
        }
    }
}
