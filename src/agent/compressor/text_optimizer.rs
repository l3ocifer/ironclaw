//! Text-level optimizations targeting tokenizer inefficiencies.
//!
//! Ported from claw-compactor's `tokenizer_optimizer.py`.

use std::sync::LazyLock;

use regex::Regex;

/// Apply all text optimizations.
pub fn optimize_text(text: &str, aggressive: bool) -> String {
    let mut result = text.to_string();
    result = normalize_cjk_punctuation(&result);
    result = minimize_whitespace(&result);
    result = compact_tables(&result);
    if aggressive {
        result = strip_trivial_backticks(&result);
        result = compact_bullets(&result);
    }
    result
}

/// Normalize CJK fullwidth punctuation to ASCII equivalents.
///
/// Each substitution saves ~1 token.
fn normalize_cjk_punctuation(text: &str) -> String {
    text.replace('，', ",")
        .replace('。', ".")
        .replace('；', ";")
        .replace('：', ":")
        .replace('？', "?")
        .replace('！', "!")
        .replace('（', "(")
        .replace('）', ")")
        .replace('【', "[")
        .replace('】', "]")
        .replace(['「', '」', '『', '』'], "\"")
        .replace('\u{3000}', " ") // Ideographic space
}

/// Minimize whitespace: collapse multiple spaces, cap indentation, collapse blank lines.
fn minimize_whitespace(text: &str) -> String {
    static MULTI_SPACE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r" {2,}").unwrap());
    static MULTI_NEWLINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\n{3,}").unwrap());

    let mut result = String::with_capacity(text.len());

    for line in text.lines() {
        // Cap leading indentation at 4 spaces
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let capped_indent = indent.min(4);
        let indented = format!("{}{}", " ".repeat(capped_indent), trimmed);

        // Collapse multiple spaces within the line
        let collapsed = MULTI_SPACE.replace_all(&indented, " ");

        // Remove trailing whitespace
        let cleaned = collapsed.trim_end();

        result.push_str(cleaned);
        result.push('\n');
    }

    // Collapse 3+ consecutive newlines to 2
    let result = MULTI_NEWLINE.replace_all(&result, "\n\n");

    // Remove trailing newline added by the loop
    result.trim_end_matches('\n').to_string() + if text.ends_with('\n') { "\n" } else { "" }
}

/// Remove backticks around simple words (not real code).
///
/// Keeps backticks around strings with spaces or special characters.
fn strip_trivial_backticks(text: &str) -> String {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"`([a-zA-Z0-9_.-]+)`").unwrap());
    RE.replace_all(text, "$1").to_string()
}

/// Compact consecutive bullet lists (3+ items).
///
/// Removes bullet prefixes from short items, joining them with commas.
fn compact_bullets(text: &str) -> String {
    static BULLET_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(\s*[-*+]\s+)(.+)$").unwrap());

    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut bullet_run: Vec<String> = Vec::new();
    let mut in_bullet_run = false;

    for line in &lines {
        if let Some(caps) = BULLET_RE.captures(line) {
            let content = caps[2].trim().to_string();
            let word_count = content.split_whitespace().count();

            if word_count <= 3 {
                // Short bullet — accumulate
                bullet_run.push(content);
                in_bullet_run = true;
            } else {
                // Long bullet — flush any accumulated short bullets
                if in_bullet_run && bullet_run.len() >= 3 {
                    result.push(bullet_run.join(", "));
                } else {
                    for b in &bullet_run {
                        result.push(format!("- {}", b));
                    }
                }
                bullet_run.clear();
                in_bullet_run = false;
                result.push(line.to_string());
            }
        } else {
            // Non-bullet line — flush accumulated bullets
            if in_bullet_run {
                if bullet_run.len() >= 3 {
                    result.push(bullet_run.join(", "));
                } else {
                    for b in &bullet_run {
                        result.push(format!("- {}", b));
                    }
                }
                bullet_run.clear();
                in_bullet_run = false;
            }
            result.push(line.to_string());
        }
    }

    // Flush remaining
    if !bullet_run.is_empty() {
        if bullet_run.len() >= 3 {
            result.push(bullet_run.join(", "));
        } else {
            for b in &bullet_run {
                result.push(format!("- {}", b));
            }
        }
    }

    result.join("\n")
}

/// Compact markdown tables.
///
/// 2-column tables → "Key: Value" format.
/// Multi-column → compact pipe format (no separator line).
fn compact_tables(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        // Detect table: header row with pipes, followed by separator row
        if lines[i].contains('|') && i + 1 < lines.len() && is_table_separator(lines[i + 1]) {
            let header = parse_table_row(lines[i]);
            let col_count = header.len();

            // Skip separator
            i += 2;

            let mut rows = Vec::new();
            while i < lines.len() && lines[i].contains('|') && !is_table_separator(lines[i]) {
                rows.push(parse_table_row(lines[i]));
                i += 1;
            }

            if col_count == 2 {
                // 2-column → Key: Value
                for row in &rows {
                    if row.len() >= 2 {
                        result.push(format!("{}: {}", row[0].trim(), row[1].trim()));
                    }
                }
            } else {
                // Multi-column → compact pipe (header + rows, no separator)
                result.push(
                    header
                        .iter()
                        .map(|h| h.trim())
                        .collect::<Vec<_>>()
                        .join(" | "),
                );
                for row in &rows {
                    result.push(
                        row.iter()
                            .map(|c| c.trim())
                            .collect::<Vec<_>>()
                            .join(" | "),
                    );
                }
            }
        } else {
            result.push(lines[i].to_string());
            i += 1;
        }
    }

    result.join("\n")
}

/// Check if a line is a markdown table separator (e.g., "| --- | --- |").
fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('|')
        && trimmed
            .chars()
            .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
}

/// Parse a markdown table row into cells.
fn parse_table_row(line: &str) -> Vec<String> {
    line.split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_cjk_punctuation() {
        let text = "价格是100元，数量是5个。";
        let result = normalize_cjk_punctuation(text);
        assert!(result.contains(','));
        assert!(result.contains('.'));
        assert!(!result.contains('，'));
    }

    #[test]
    fn test_minimize_whitespace() {
        let text = "hello   world\n\n\n\nfoo  bar";
        let result = minimize_whitespace(text);
        assert!(!result.contains("   ")); // No triple spaces
        assert!(!result.contains("\n\n\n")); // No triple newlines
    }

    #[test]
    fn test_strip_trivial_backticks() {
        let text = "Use `cargo` to build and `npm` to install `my complex package`";
        let result = strip_trivial_backticks(text);
        assert_eq!(result, "Use cargo to build and npm to install `my complex package`");
    }

    #[test]
    fn test_compact_bullets_short() {
        let text = "- A\n- B\n- C\n- D";
        let result = compact_bullets(text);
        assert_eq!(result, "A, B, C, D");
    }

    #[test]
    fn test_compact_bullets_long_preserved() {
        let text = "- This is a long bullet point with many words\n- Another long one here";
        let result = compact_bullets(text);
        assert!(result.contains("- This is a long"));
    }

    #[test]
    fn test_compact_tables_two_column() {
        let text = "| Key | Value |\n| --- | --- |\n| Name | Alice |\n| Age | 30 |";
        let result = compact_tables(text);
        assert!(result.contains("Name: Alice"));
        assert!(result.contains("Age: 30"));
    }

    #[test]
    fn test_compact_tables_multi_column() {
        let text = "| A | B | C |\n| --- | --- | --- |\n| 1 | 2 | 3 |";
        let result = compact_tables(text);
        assert!(result.contains("A | B | C"));
        assert!(result.contains("1 | 2 | 3"));
    }

    #[test]
    fn test_is_table_separator() {
        assert!(is_table_separator("| --- | --- |"));
        assert!(is_table_separator("|---|---|"));
        assert!(is_table_separator("| :---: | ---: |"));
        assert!(!is_table_separator("| hello | world |"));
    }
}
