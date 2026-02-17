//! Salience scoring for conversation turns and sessions.
//!
//! Inspired by contrail's salience scoring. Assigns importance weights to
//! turns based on content signals (errors, questions, file effects, etc.)
//! and applies recency boosts to sessions.

use chrono::{DateTime, Utc};

/// Salience cues detected in a turn.
#[derive(Debug, Clone)]
pub struct SalienceResult {
    pub score: f32,
    pub cues: Vec<String>,
}

/// Score a single conversation turn by content analysis.
///
/// Higher scores indicate more important turns that should be preserved
/// during compaction rather than summarized.
pub fn score_turn(content: &str, role: &str) -> SalienceResult {
    let mut score: f32 = 1.0;
    let mut cues = Vec::new();
    let lower = content.to_lowercase();

    // User messages are slightly more important (they represent intent)
    if role == "user" {
        score += 0.3;
    }

    // Questions indicate information-seeking (preserve)
    if lower.contains('?') {
        score += 0.4;
        cues.push("question".into());
    }

    // Errors and failures are high-value (preserve for debugging context)
    if contains_any(&lower, &["error", "fail", "panic", "exception", "stack trace", "traceback"]) {
        score += 0.6;
        cues.push("error".into());
    }

    // TODOs and action items
    if lower.contains("todo") || lower.contains("fixme") || lower.contains("hack") {
        score += 0.2;
        cues.push("todo".into());
    }

    // Decisions and conclusions
    if contains_any(&lower, &["decided", "conclusion", "solution", "resolved", "fixed"]) {
        score += 0.4;
        cues.push("decision".into());
    }

    // Code changes / file operations
    if contains_any(&lower, &["created file", "modified", "deleted", "wrote to", "saved to"]) {
        score += 0.5;
        cues.push("file_effect".into());
    }

    // Memory operations (durable knowledge)
    if contains_any(&lower, &["memory_write", "memory_append", "learning_create"]) {
        score += 0.3;
        cues.push("memory_op".into());
    }

    // Long messages tend to be more substantive
    if content.len() > 800 {
        score += 0.2;
        cues.push("long".into());
    }

    // Very short messages are less important
    if content.len() < 20 && role == "assistant" {
        score -= 0.3;
        cues.push("brief".into());
    }

    // Tool calls are important (they represent actions)
    if role == "tool" {
        score += 0.2;
        cues.push("tool_result".into());
    }

    SalienceResult { score, cues }
}

/// Apply a recency boost to a session-level score.
///
/// More recent sessions get a multiplicative boost that decays with age.
/// `ended_at` is when the session ended; `now` is the current time.
pub fn recency_boost(ended_at: DateTime<Utc>, now: DateTime<Utc>) -> f32 {
    let age_days = (now - ended_at).num_seconds().abs() as f32 / 86_400.0;
    1.0 + (0.5 / (1.0 + age_days))
}

/// Rank turns by salience and return indices of the most important ones.
///
/// Returns indices sorted by descending salience score, limited to `max_count`.
pub fn rank_turns(turns: &[(String, String)], max_count: usize) -> Vec<usize> {
    let mut scored: Vec<(usize, f32)> = turns
        .iter()
        .enumerate()
        .map(|(i, (content, role))| {
            let result = score_turn(content, role);
            (i, result.score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(max_count).map(|(i, _)| i).collect()
}

/// Partition turns into "keep verbatim" and "summarize" based on salience threshold.
///
/// Returns (keep_indices, summarize_indices) where keep has salience >= threshold.
pub fn partition_by_salience(
    turns: &[(String, String)],
    threshold: f32,
) -> (Vec<usize>, Vec<usize>) {
    let mut keep = Vec::new();
    let mut summarize = Vec::new();

    for (i, (content, role)) in turns.iter().enumerate() {
        let result = score_turn(content, role);
        if result.score >= threshold {
            keep.push(i);
        } else {
            summarize.push(i);
        }
    }

    (keep, summarize)
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_message_scores_higher() {
        let user = score_turn("hello world", "user");
        let assistant = score_turn("hello world", "assistant");
        assert!(user.score > assistant.score);
    }

    #[test]
    fn test_error_message_scores_high() {
        let result = score_turn("Error: connection failed with panic", "assistant");
        assert!(result.score > 1.5);
        assert!(result.cues.contains(&"error".to_string()));
    }

    #[test]
    fn test_question_detected() {
        let result = score_turn("How should we handle this?", "user");
        assert!(result.cues.contains(&"question".to_string()));
    }

    #[test]
    fn test_decision_detected() {
        let result = score_turn("We decided to use PostgreSQL for this", "assistant");
        assert!(result.cues.contains(&"decision".to_string()));
    }

    #[test]
    fn test_recency_boost_recent() {
        let now = Utc::now();
        let boost = recency_boost(now, now);
        assert!(boost > 1.4); // Very recent = high boost
    }

    #[test]
    fn test_recency_boost_old() {
        let now = Utc::now();
        let old = now - chrono::Duration::days(30);
        let boost = recency_boost(old, now);
        assert!(boost < 1.1); // Old = low boost
    }

    #[test]
    fn test_rank_turns() {
        let turns = vec![
            ("hello".into(), "user".into()),
            ("Error: panic at line 42".into(), "assistant".into()),
            ("ok".into(), "assistant".into()),
            ("How do we fix this?".into(), "user".into()),
        ];
        let ranked = rank_turns(&turns, 2);
        assert_eq!(ranked.len(), 2);
        // Error and question should rank highest
        assert!(ranked.contains(&1) || ranked.contains(&3));
    }

    #[test]
    fn test_partition_by_salience() {
        let turns = vec![
            ("hello".into(), "assistant".into()),
            ("Error: critical failure".into(), "assistant".into()),
        ];
        let (keep, summarize) = partition_by_salience(&turns, 1.5);
        assert!(keep.contains(&1)); // error should be kept
        assert!(summarize.contains(&0)); // hello should be summarized
    }
}
