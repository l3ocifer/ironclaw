//! Observation extraction from conversation messages.
//!
//! Ported from claw-compactor's `observation_compressor.py`.
//!
//! Converts raw conversation message sequences into structured observation
//! summaries. This is the highest-savings compression layer (~97% on session
//! transcripts) because it distills verbose tool call/response pairs and
//! multi-turn reasoning into compact factual statements.
//!
//! # Categories
//!
//! Observations are classified into:
//! - **Decisions**: Choices the user or agent made ("chose X over Y")
//! - **Actions**: Things that were done ("ran tests", "deployed to staging")
//! - **Facts**: Information discovered ("API returns 404", "file has 200 lines")
//! - **Errors**: Failures encountered ("build failed: missing dep")
//! - **Context**: Environmental state ("working dir is /foo", "using Rust 1.92")

use std::collections::HashMap;

use crate::llm::ChatMessage;

/// An extracted observation from a conversation.
#[derive(Debug, Clone)]
pub struct Observation {
    /// Category of the observation.
    pub category: ObservationCategory,
    /// The observation text.
    pub text: String,
    /// Approximate turn number where this was observed.
    pub turn: usize,
}

/// Category of an observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservationCategory {
    Decision,
    Action,
    Fact,
    Error,
    Context,
}

impl ObservationCategory {
    fn label(&self) -> &'static str {
        match self {
            Self::Decision => "Decisions",
            Self::Action => "Actions",
            Self::Fact => "Facts",
            Self::Error => "Errors",
            Self::Context => "Context",
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            Self::Decision => "→",
            Self::Action => "✓",
            Self::Fact => "•",
            Self::Error => "✗",
            Self::Context => "◦",
        }
    }
}

/// Extract observations from a sequence of messages.
///
/// Uses heuristic pattern matching (no LLM call) to identify key facts,
/// decisions, actions, and errors from the conversation. This is a
/// deterministic, fast operation.
pub fn extract_observations(messages: &[ChatMessage]) -> Vec<Observation> {
    let mut observations = Vec::new();
    let mut turn = 0;

    for (i, msg) in messages.iter().enumerate() {
        if msg.role == crate::llm::Role::User {
            turn += 1;
        }

        let content = &msg.content;
        if content.is_empty() {
            continue;
        }

        match msg.role {
            crate::llm::Role::User => {
                extract_user_observations(content, turn, &mut observations);
            }
            crate::llm::Role::Assistant => {
                extract_assistant_observations(content, turn, &mut observations);
            }
            crate::llm::Role::Tool => {
                let tool_name = msg.name.as_deref().unwrap_or("unknown");
                extract_tool_observations(content, tool_name, turn, &mut observations);
            }
            crate::llm::Role::System => {
                // System messages contain context info
                if i == 0 {
                    extract_context_observations(content, turn, &mut observations);
                }
            }
        }
    }

    deduplicate_observations(observations)
}

/// Format observations into a compact markdown summary.
pub fn format_observations(observations: &[Observation]) -> String {
    if observations.is_empty() {
        return String::new();
    }

    let mut by_category: HashMap<ObservationCategory, Vec<&Observation>> = HashMap::new();
    for obs in observations {
        by_category.entry(obs.category).or_default().push(obs);
    }

    let mut parts = Vec::new();

    // Output in a stable order
    let order = [
        ObservationCategory::Decision,
        ObservationCategory::Action,
        ObservationCategory::Error,
        ObservationCategory::Fact,
        ObservationCategory::Context,
    ];

    for cat in &order {
        if let Some(items) = by_category.get(cat) {
            parts.push(format!("**{}**", cat.label()));
            for obs in items {
                parts.push(format!("{} {}", cat.emoji(), obs.text));
            }
        }
    }

    parts.join("\n")
}

/// Replace verbose messages with their observation summary.
///
/// Returns a single summary message replacing the original messages,
/// plus any messages that couldn't be summarized (kept as-is).
pub fn compress_to_observations(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    if messages.is_empty() {
        return Vec::new();
    }

    let observations = extract_observations(messages);
    if observations.is_empty() {
        return messages.to_vec();
    }

    let summary = format_observations(&observations);
    if summary.is_empty() {
        return messages.to_vec();
    }

    // Keep system messages and the most recent user + assistant messages
    let mut result = Vec::new();

    // Preserve system messages
    for msg in messages {
        if msg.role == crate::llm::Role::System {
            result.push(msg.clone());
        }
    }

    // Add the observation summary as a system message
    result.push(ChatMessage::system(&format!(
        "## Session Observations\n\n{}",
        summary
    )));

    // Keep the last user message and its response for continuity
    let mut last_user_idx = None;
    for (i, msg) in messages.iter().enumerate().rev() {
        if msg.role == crate::llm::Role::User {
            last_user_idx = Some(i);
            break;
        }
    }
    if let Some(idx) = last_user_idx {
        for msg in &messages[idx..] {
            result.push(msg.clone());
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Heuristic extraction functions
// ---------------------------------------------------------------------------

fn extract_user_observations(content: &str, turn: usize, out: &mut Vec<Observation>) {
    let lower = content.to_lowercase();

    // Decision indicators
    if lower.contains("let's go with")
        || lower.contains("i choose")
        || lower.contains("let's use")
        || lower.contains("we should")
        || lower.contains("go ahead with")
        || lower.contains("yes, let's")
        || lower.contains("i prefer")
    {
        // Extract the first sentence as the decision
        if let Some(sentence) = first_sentence(content) {
            out.push(Observation {
                category: ObservationCategory::Decision,
                text: truncate_str(&sentence, 120),
                turn,
            });
        }
    }

    // Action requests
    if lower.contains("please ")
        || lower.contains("can you ")
        || lower.contains("run ")
        || lower.contains("create ")
        || lower.contains("update ")
        || lower.contains("fix ")
        || lower.contains("add ")
        || lower.contains("implement ")
    {
        if let Some(sentence) = first_sentence(content) {
            if sentence.len() > 10 {
                out.push(Observation {
                    category: ObservationCategory::Action,
                    text: format!("User requested: {}", truncate_str(&sentence, 100)),
                    turn,
                });
            }
        }
    }
}

fn extract_assistant_observations(content: &str, turn: usize, out: &mut Vec<Observation>) {
    let lines: Vec<&str> = content.lines().collect();

    for line in &lines {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        // Error patterns
        if lower.contains("error:")
            || lower.contains("failed:")
            || lower.contains("failure:")
            || lower.starts_with("error")
            || lower.contains("exception:")
            || lower.contains("could not")
            || lower.contains("unable to")
        {
            out.push(Observation {
                category: ObservationCategory::Error,
                text: truncate_str(trimmed, 150),
                turn,
            });
            continue;
        }

        // Action completion patterns
        if lower.starts_with("created ")
            || lower.starts_with("updated ")
            || lower.starts_with("deleted ")
            || lower.starts_with("installed ")
            || lower.starts_with("deployed ")
            || lower.starts_with("fixed ")
            || lower.starts_with("added ")
            || lower.starts_with("removed ")
            || lower.starts_with("configured ")
            || lower.starts_with("wrote ")
            || lower.starts_with("ran ")
        {
            out.push(Observation {
                category: ObservationCategory::Action,
                text: truncate_str(trimmed, 120),
                turn,
            });
        }

        // Decision patterns from assistant
        if lower.contains("i'll use ")
            || lower.contains("i recommend ")
            || lower.contains("the best approach")
            || lower.contains("decided to")
        {
            out.push(Observation {
                category: ObservationCategory::Decision,
                text: truncate_str(trimmed, 120),
                turn,
            });
        }
    }
}

fn extract_tool_observations(
    content: &str,
    tool_name: &str,
    turn: usize,
    out: &mut Vec<Observation>,
) {
    let lower = content.to_lowercase();

    // Tool errors
    if lower.contains("error") || lower.contains("failed") || lower.contains("not found") {
        out.push(Observation {
            category: ObservationCategory::Error,
            text: format!(
                "Tool `{}`: {}",
                tool_name,
                truncate_str(&first_line(content), 120)
            ),
            turn,
        });
        return;
    }

    // Successful tool results — extract first meaningful line
    match tool_name {
        "shell" => {
            // Shell output: capture exit code or first line
            if let Some(line) = content.lines().next() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && trimmed.len() < 200 {
                    out.push(Observation {
                        category: ObservationCategory::Fact,
                        text: format!("Shell: {}", truncate_str(trimmed, 100)),
                        turn,
                    });
                }
            }
        }
        "read_file" | "memory_read" => {
            out.push(Observation {
                category: ObservationCategory::Action,
                text: format!("Read file via `{}`", tool_name),
                turn,
            });
        }
        "write_file" | "memory_write" => {
            out.push(Observation {
                category: ObservationCategory::Action,
                text: format!("Wrote file via `{}`", tool_name),
                turn,
            });
        }
        "memory_search" => {
            let result_count = content.matches("score:").count();
            out.push(Observation {
                category: ObservationCategory::Fact,
                text: format!("Memory search returned {} results", result_count),
                turn,
            });
        }
        _ => {
            // Generic tool observation
            if content.len() < 100 {
                out.push(Observation {
                    category: ObservationCategory::Fact,
                    text: format!("Tool `{}`: {}", tool_name, truncate_str(content, 80)),
                    turn,
                });
            } else {
                out.push(Observation {
                    category: ObservationCategory::Action,
                    text: format!("Used tool `{}`", tool_name),
                    turn,
                });
            }
        }
    }
}

fn extract_context_observations(content: &str, turn: usize, out: &mut Vec<Observation>) {
    // Extract key context from system prompt
    for line in content.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        // Working directory
        if lower.contains("working directory") || lower.contains("workspace:") {
            out.push(Observation {
                category: ObservationCategory::Context,
                text: truncate_str(trimmed, 100),
                turn,
            });
        }

        // Agent identity
        if lower.contains("you are ") && trimmed.len() < 100 {
            out.push(Observation {
                category: ObservationCategory::Context,
                text: truncate_str(trimmed, 100),
                turn,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Deduplicate observations by text similarity (exact match on lowercased text).
fn deduplicate_observations(observations: Vec<Observation>) -> Vec<Observation> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for obs in observations {
        let key = obs.text.to_lowercase();
        if seen.insert(key) {
            result.push(obs);
        }
    }
    result
}

/// Get the first sentence from text (up to first period, exclamation, or question mark).
fn first_sentence(text: &str) -> Option<String> {
    let first_line = text.lines().next()?;
    let trimmed = first_line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Find sentence boundary
    for (i, ch) in trimmed.char_indices() {
        if (ch == '.' || ch == '!' || ch == '?') && i > 5 {
            return Some(trimmed[..=i].to_string());
        }
    }

    // No sentence boundary found — use the whole first line
    Some(trimmed.to_string())
}

/// Get the first non-empty line.
fn first_line(text: &str) -> String {
    text.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Truncate a string to max characters, appending "..." if truncated.
fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let boundary = s
            .char_indices()
            .nth(max.saturating_sub(3))
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        format!("{}...", &s[..boundary])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_user_decision() {
        let msgs = vec![ChatMessage::user(
            "Let's go with PostgreSQL for the database.",
        )];
        let obs = extract_observations(&msgs);
        assert!(obs.iter().any(|o| o.category == ObservationCategory::Decision));
    }

    #[test]
    fn test_extract_error_from_assistant() {
        let msgs = vec![ChatMessage::assistant(
            "The build failed. Error: missing dependency `tokio`.",
        )];
        let obs = extract_observations(&msgs);
        assert!(obs.iter().any(|o| o.category == ObservationCategory::Error));
    }

    #[test]
    fn test_extract_action_from_assistant() {
        let msgs = vec![ChatMessage::assistant(
            "Created the new file at src/main.rs with the entry point.",
        )];
        let obs = extract_observations(&msgs);
        assert!(obs.iter().any(|o| o.category == ObservationCategory::Action));
    }

    #[test]
    fn test_format_observations() {
        let obs = vec![
            Observation {
                category: ObservationCategory::Decision,
                text: "Chose PostgreSQL".to_string(),
                turn: 1,
            },
            Observation {
                category: ObservationCategory::Error,
                text: "Build failed: missing dep".to_string(),
                turn: 2,
            },
        ];
        let formatted = format_observations(&obs);
        assert!(formatted.contains("**Decisions**"));
        assert!(formatted.contains("Chose PostgreSQL"));
        assert!(formatted.contains("**Errors**"));
    }

    #[test]
    fn test_compress_to_observations_preserves_system() {
        let msgs = vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("Let's go with Rust for this."),
            ChatMessage::assistant("Created the project structure."),
        ];
        let compressed = compress_to_observations(&msgs);
        assert!(compressed.iter().any(|m| m.role == crate::llm::Role::System));
        // Should have observation summary + preserved messages
        assert!(compressed.len() <= msgs.len() + 1);
    }

    #[test]
    fn test_empty_messages() {
        let obs = extract_observations(&[]);
        assert!(obs.is_empty());
        let formatted = format_observations(&obs);
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_deduplicate_observations() {
        let obs = vec![
            Observation {
                category: ObservationCategory::Fact,
                text: "Same thing".to_string(),
                turn: 1,
            },
            Observation {
                category: ObservationCategory::Fact,
                text: "Same thing".to_string(),
                turn: 2,
            },
        ];
        let deduped = deduplicate_observations(obs);
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("hello world this is long", 10), "hello w...");
    }
}
