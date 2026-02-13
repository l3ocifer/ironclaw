//! Pattern-based compression: paths, IPs, enumerations, repeated headers.
//!
//! Ported from claw-compactor's `rle.py`.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

/// Apply all pattern compressions to a text.
pub fn compress_patterns(text: &str) -> String {
    let mut result = text.to_string();
    result = compress_repeated_headers(&result);
    result = compress_enumerations(&result);
    result = remove_duplicate_lines(&result);
    result
}

/// Remove exact duplicate lines (preserve order, keep first occurrence).
fn remove_duplicate_lines(text: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut lines = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        // Always keep blank lines and lines that are just whitespace
        if trimmed.is_empty() {
            lines.push(line.to_string());
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            lines.push(line.to_string());
        }
    }

    lines.join("\n")
}

/// Compress comma-separated ALL-CAPS enumerations (4+ items).
///
/// "BTC, ETH, SOL, BNB, AVAX" → "[BTC,ETH,SOL,BNB,AVAX]"
fn compress_enumerations(text: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"((?:[A-Z][A-Z0-9]{1,6})(?:\s*,\s*(?:[A-Z][A-Z0-9]{1,6})){3,})").unwrap()
    });

    RE.replace_all(text, |caps: &regex::Captures| {
        let items: Vec<&str> = caps[1].split(',').map(|s| s.trim()).collect();
        format!("[{}]", items.join(","))
    })
    .to_string()
}

/// Remove duplicate markdown headers, merging their content.
///
/// If the same header appears multiple times, keep only the first occurrence
/// and append the content of subsequent occurrences below it.
fn compress_repeated_headers(text: &str) -> String {
    static HEADER_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(#{1,6})\s+(.+)$").unwrap());

    let mut seen_headers: HashMap<String, usize> = HashMap::new();
    let mut result_lines: Vec<String> = Vec::new();
    let mut skip_until_next_header = false;
    let mut pending_content: Vec<String> = Vec::new();
    let mut merge_target: Option<usize> = None;

    for line in text.lines() {
        if let Some(caps) = HEADER_RE.captures(line) {
            let _level = caps[1].len();
            let title = caps[2].trim().to_string();
            let key = title.to_lowercase();

            if let Some(&first_idx) = seen_headers.get(&key) {
                // Duplicate header — skip it, collect content to merge
                skip_until_next_header = true;
                merge_target = Some(first_idx);
                pending_content.clear();
                continue;
            }

            // First occurrence — record position
            if skip_until_next_header {
                // We hit a new header after skipping a duplicate section
                // Flush pending content to the merge target
                if let Some(target_idx) = merge_target {
                    // Find where to insert (after the target header's section)
                    // Simple approach: just append to the end of result
                    for pc in &pending_content {
                        result_lines.insert(target_idx + 1, pc.clone());
                    }
                }
                skip_until_next_header = false;
                merge_target = None;
                pending_content.clear();
            }

            seen_headers.insert(key, result_lines.len());
            result_lines.push(line.to_string());
        } else if skip_until_next_header {
            // Content under a duplicate header — collect for merging
            pending_content.push(line.to_string());
        } else {
            result_lines.push(line.to_string());
        }
    }

    // Flush any remaining pending content
    if let Some(target_idx) = merge_target {
        let insert_at = (target_idx + 1).min(result_lines.len());
        for (i, pc) in pending_content.iter().enumerate() {
            result_lines.insert(insert_at + i, pc.clone());
        }
    }

    result_lines.join("\n")
}

/// Compress long file paths by detecting common prefixes.
///
/// Scans text for paths (containing at least 3 `/` separators) and
/// replaces repeated prefixes with `$PATH_N` variables.
pub fn compress_paths(text: &str, workspace_paths: &[&str]) -> String {
    let mut result = text.to_string();

    // Sort by length descending (longest prefix first)
    let mut sorted_paths = workspace_paths.to_vec();
    sorted_paths.sort_by_key(|b| std::cmp::Reverse(b.len()));

    for (i, path) in sorted_paths.iter().enumerate() {
        if result.contains(*path) {
            let var = format!("$WS{}", if i == 0 { String::new() } else { i.to_string() });
            result = result.replace(*path, &var);
        }
    }

    result
}

/// Compress IP address families by grouping common prefixes.
///
/// Groups IPs by first 3 octets. If 2+ IPs share a prefix, replaces with
/// `$IP_N.last_octet`.
pub fn compress_ip_families(text: &str) -> (String, HashMap<String, String>) {
    static IP_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\b(\d{1,3}\.\d{1,3}\.\d{1,3})\.(\d{1,3})\b").unwrap()
    });

    // Count prefix occurrences
    let mut prefix_counts: HashMap<String, usize> = HashMap::new();
    for caps in IP_RE.captures_iter(text) {
        let prefix = caps[1].to_string();
        *prefix_counts.entry(prefix).or_insert(0) += 1;
    }

    // Only compress families with 2+ members
    let compress_prefixes: Vec<(String, String)> = prefix_counts
        .iter()
        .filter(|(_, count)| **count >= 2)
        .enumerate()
        .map(|(i, (prefix, _))| {
            let var = format!("$IP{}", if i == 0 { String::new() } else { i.to_string() });
            (prefix.clone(), var)
        })
        .collect();

    if compress_prefixes.is_empty() {
        return (text.to_string(), HashMap::new());
    }

    let mut result = text.to_string();
    let mut prefix_map = HashMap::new();

    for (prefix, var) in &compress_prefixes {
        // Replace "PREFIX.LAST" with "VAR.LAST"
        let pattern = format!(r"{}\.", regex::escape(prefix));
        if let Ok(re) = Regex::new(&pattern) {
            result = re.replace_all(&result, format!("{}.", var)).to_string();
        }
        prefix_map.insert(var.clone(), prefix.clone());
    }

    (result, prefix_map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_duplicate_lines() {
        let text = "line one\nline two\nline one\nline three";
        let result = remove_duplicate_lines(text);
        assert_eq!(result, "line one\nline two\nline three");
    }

    #[test]
    fn test_remove_duplicate_lines_preserves_blanks() {
        let text = "line one\n\nline two\n\nline one";
        let result = remove_duplicate_lines(text);
        // Blank lines are preserved, duplicate "line one" is removed
        assert_eq!(result, "line one\n\nline two\n");
    }

    #[test]
    fn test_compress_enumerations() {
        let text = "Tokens: BTC, ETH, SOL, BNB, AVAX are popular";
        let result = compress_enumerations(text);
        assert!(result.contains("[BTC,ETH,SOL,BNB,AVAX]"));
    }

    #[test]
    fn test_compress_enumerations_short_list_unchanged() {
        let text = "Options: A, B, C"; // Only 3 items, needs 4+
        let result = compress_enumerations(text);
        assert_eq!(result, text);
    }

    #[test]
    fn test_compress_paths() {
        let text = "Read /Users/leo/projects/myapp/src/main.rs and /Users/leo/projects/myapp/Cargo.toml";
        let result = compress_paths(text, &["/Users/leo/projects/myapp"]);
        assert!(result.contains("$WS/src/main.rs"));
        assert!(result.contains("$WS/Cargo.toml"));
    }

    #[test]
    fn test_compress_ip_families() {
        let text = "Server 192.168.1.100 and 192.168.1.200 are in the same subnet, but 10.0.0.1 is alone";
        let (result, prefix_map) = compress_ip_families(text);
        // The 192.168.1.x family should be compressed
        assert!(prefix_map.values().any(|v| v == "192.168.1"));
        assert!(result.contains("10.0.0.1")); // Single IP unchanged
    }
}
