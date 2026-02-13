//! Logseq graph loader for bootstrap memory injection.
//!
//! Reads from a configured Logseq graph path under strict security:
//! - All paths must be under the canonical graph_path (no escape).
//! - Only .md files; symlinks are skipped.
//! - Character limit enforced to cap token usage.

use std::path::Path;

use crate::config::LogseqConfig;

/// Approximate characters per token for truncation.
const CHARS_PER_TOKEN: usize = 4;

/// Load Logseq context for bootstrap: user profile, agent preferences, recent decisions.
///
/// Returns a string suitable to prepend to MEMORY.md content. Empty if path invalid or no content.
pub fn load_logseq_context(config: &LogseqConfig, agent_name: &str) -> String {
    let graph_path = match config.graph_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            tracing::debug!("Logseq graph_path does not exist or not accessible: {:?}", config.graph_path);
            return String::new();
        }
    };

    let max_chars = config.max_tokens.saturating_mul(CHARS_PER_TOKEN);
    let mut parts = Vec::new();

    // User profile: shared/User.md (generic name so multiple agents can share).
    if config.include_user_profile {
        let rel = Path::new("pages")
            .join(&config.ai_namespace)
            .join("shared")
            .join("User.md");
        if let Some(s) = read_file_under(&graph_path, &rel, 1000) {
            parts.push(format!("## User Profile\n\n{}", s));
        }
    }

    // Agent-specific preferences and decisions (use agent_name from config).
    if config.include_preferences {
        let rel = Path::new("pages")
            .join(&config.ai_namespace)
            .join(agent_name)
            .join("preferences.md");
        if let Some(s) = read_file_under(&graph_path, &rel, 500) {
            parts.push(format!("## {}'s Learned Preferences\n\n{}", agent_name, s));
        }
    }

    if config.include_decisions {
        let rel = Path::new("pages")
            .join(&config.ai_namespace)
            .join(agent_name)
            .join("decisions.md");
        if let Some(s) = read_file_under(&graph_path, &rel, 800) {
            // Last 10 bullet lines (most recent at end)
            let lines: Vec<&str> = s.lines().filter(|l| l.trim().starts_with('-')).collect();
            let recent: Vec<&str> = lines.into_iter().rev().take(10).rev().collect();
            if !recent.is_empty() {
                parts.push(format!("## Recent Decisions\n\n{}", recent.join("\n")));
            }
        }
    }

    let mut out = parts.join("\n\n");
    if out.len() > max_chars {
        out.truncate(max_chars);
        out.push_str("\n[...truncated]");
    }
    out
}

/// Read a file under `base` at `relative` path, enforcing path containment and no symlinks.
/// Truncate to `max_chars`. Returns None if path escapes, is symlink, or read fails.
fn read_file_under(base: &Path, relative: &Path, max_chars: usize) -> Option<String> {
    let full = base.join(relative);
    if !full.is_file() {
        return None;
    }
    // Refuse symlinks (symlink_metadata does not follow).
    if full.symlink_metadata().ok()?.file_type().is_symlink() {
        return None;
    }
    let canonical = full.canonicalize().ok()?;
    if !canonical.starts_with(base) {
        return None;
    }
    let content = std::fs::read_to_string(&canonical).ok()?;
    let truncated = if content.len() > max_chars {
        format!("{}[...truncated]", &content[..max_chars])
    } else {
        content
    };
    Some(truncated)
}
