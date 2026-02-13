//! Near-duplicate message detection via shingle hashing + Jaccard similarity.
//!
//! Ported from claw-compactor's `dedup.py`.

use std::collections::HashSet;

use crate::llm::ChatMessage;

/// Generate k-word shingle hashes from text.
///
/// Each shingle is a hash of `k` consecutive words.
fn shingles(text: &str, k: usize) -> HashSet<u64> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < k {
        // For very short texts, use the whole text as one shingle
        let mut set = HashSet::new();
        set.insert(hash_str(&words.join(" ")));
        return set;
    }

    let mut set = HashSet::with_capacity(words.len().saturating_sub(k) + 1);
    for window in words.windows(k) {
        set.insert(hash_str(&window.join(" ")));
    }
    set
}

/// Simple string hash (FNV-1a inspired, stable across runs).
fn hash_str(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Compute Jaccard similarity between two shingle sets.
///
/// Returns 1.0 for identical sets, 0.0 for completely disjoint sets.
fn jaccard(a: &HashSet<u64>, b: &HashSet<u64>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

/// Remove near-duplicate messages from a list.
///
/// Keeps the first occurrence of each group of similar messages.
/// Messages with different roles are never considered duplicates.
pub fn deduplicate_messages(
    messages: &[ChatMessage],
    threshold: f64,
    shingle_size: usize,
) -> Vec<ChatMessage> {
    if messages.len() <= 1 {
        return messages.to_vec();
    }

    // Pre-compute shingles for all messages
    let message_shingles: Vec<HashSet<u64>> = messages
        .iter()
        .map(|m| shingles(&m.content, shingle_size))
        .collect();

    // Track which messages to keep (indices)
    let mut keep = vec![true; messages.len()];

    for i in 0..messages.len() {
        if !keep[i] {
            continue;
        }
        for j in (i + 1)..messages.len() {
            if !keep[j] {
                continue;
            }
            // Only compare messages with the same role
            if messages[i].role != messages[j].role {
                continue;
            }
            // Don't dedup system messages
            if messages[i].role == crate::llm::Role::System {
                continue;
            }

            let sim = jaccard(&message_shingles[i], &message_shingles[j]);
            if sim >= threshold {
                // Keep the longer message (more information)
                if messages[i].content.len() >= messages[j].content.len() {
                    keep[j] = false;
                } else {
                    keep[i] = false;
                    break; // i is removed, stop comparing
                }
            }
        }
    }

    messages
        .iter()
        .enumerate()
        .filter(|(idx, _)| keep[*idx])
        .map(|(_, m)| m.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shingles_basic() {
        let s = shingles("the quick brown fox jumps", 3);
        assert!(!s.is_empty());
        // 5 words, k=3 → 3 shingles
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn test_jaccard_identical() {
        let a = shingles("hello world foo bar", 2);
        let b = shingles("hello world foo bar", 2);
        assert!((jaccard(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_jaccard_different() {
        let a = shingles("hello world foo bar", 2);
        let b = shingles("completely unrelated text here", 2);
        assert!(jaccard(&a, &b) < 0.3);
    }

    #[test]
    fn test_dedup_identical_messages() {
        let messages = vec![
            ChatMessage::user("Please help me with my code"),
            ChatMessage::assistant("Sure, I can help!"),
            ChatMessage::user("Please help me with my code"), // duplicate
        ];

        let result = deduplicate_messages(&messages, 0.6, 3);
        assert_eq!(result.len(), 2); // One user msg removed
    }

    #[test]
    fn test_dedup_different_roles_kept() {
        let messages = vec![
            ChatMessage::user("Hello world test message"),
            ChatMessage::assistant("Hello world test message"), // Same content, different role
        ];

        let result = deduplicate_messages(&messages, 0.6, 3);
        assert_eq!(result.len(), 2); // Both kept (different roles)
    }

    #[test]
    fn test_dedup_near_duplicates() {
        let messages = vec![
            ChatMessage::user("The quick brown fox jumps over the lazy dog"),
            ChatMessage::user("The quick brown fox jumps over the lazy cat"),
        ];

        let result = deduplicate_messages(&messages, 0.6, 3);
        // These are very similar, should be deduped
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_dedup_preserves_system_messages() {
        let messages = vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::system("You are a helpful assistant."),
        ];

        let result = deduplicate_messages(&messages, 0.6, 3);
        assert_eq!(result.len(), 2); // System messages not deduped
    }
}
