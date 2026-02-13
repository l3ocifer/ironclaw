//! Token compression pipeline for context window optimization.
//!
//! Ported from [claw-compactor](https://github.com/aeromomo/claw-compactor)
//! (Python) to Rust. All algorithms are deterministic and require no LLM calls.
//!
//! # Pipeline
//!
//! 1. **Deduplication** — shingle hashing + Jaccard similarity to detect and remove
//!    near-duplicate messages.
//! 2. **Dictionary compression** — auto-learned codebook from repeated n-gram patterns.
//!    Common phrases are replaced with `$XX` codes.
//! 3. **Pattern compression** — path shorthand, IP prefix compression, enum compaction,
//!    repeated header merging.
//! 4. **Text optimization** — whitespace normalization, CJK punctuation normalization,
//!    table compaction, bullet merging.
//!
//! # Usage
//!
//! ```ignore
//! use ironclaw::agent::compressor::CompressorPipeline;
//!
//! let pipeline = CompressorPipeline::default();
//! let compressed = pipeline.compress(&messages);
//! ```

pub mod dedup;
pub mod dictionary;
pub mod patterns;
pub mod text_optimizer;

use crate::llm::ChatMessage;

/// Results of a compression pass.
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// Compressed messages.
    pub messages: Vec<ChatMessage>,
    /// Estimated tokens before compression.
    pub tokens_before: usize,
    /// Estimated tokens after compression.
    pub tokens_after: usize,
    /// Compression ratio (0.0–1.0, lower is more compressed).
    pub ratio: f64,
    /// Per-stage savings breakdown.
    pub stages: Vec<StageSavings>,
}

/// Savings from a single compression stage.
#[derive(Debug, Clone)]
pub struct StageSavings {
    pub name: String,
    pub tokens_saved: usize,
}

/// Configuration for the compression pipeline.
#[derive(Debug, Clone)]
pub struct CompressorConfig {
    /// Jaccard similarity threshold for dedup (0.0–1.0, default 0.6).
    pub dedup_threshold: f64,
    /// Shingle size for dedup (default 3).
    pub shingle_size: usize,
    /// Minimum frequency for dictionary entries (default 3).
    pub dict_min_freq: usize,
    /// Maximum dictionary entries (default 200).
    pub dict_max_entries: usize,
    /// Whether to apply text optimizations (default true).
    pub text_optimize: bool,
    /// Whether to apply aggressive text optimizations (default false).
    pub text_optimize_aggressive: bool,
}

impl Default for CompressorConfig {
    fn default() -> Self {
        Self {
            dedup_threshold: 0.6,
            shingle_size: 3,
            dict_min_freq: 3,
            dict_max_entries: 200,
            text_optimize: true,
            text_optimize_aggressive: false,
        }
    }
}

/// Full compression pipeline.
pub struct CompressorPipeline {
    config: CompressorConfig,
}

impl CompressorPipeline {
    pub fn new(config: CompressorConfig) -> Self {
        Self { config }
    }

    /// Compress a list of messages through all stages.
    pub fn compress(&self, messages: &[ChatMessage]) -> CompressionResult {
        let tokens_before = estimate_tokens_batch(messages);
        let mut stages = Vec::new();
        let mut current = messages.to_vec();

        // Stage 1: Deduplication
        let before_dedup = estimate_tokens_batch(&current);
        let dedup_result = dedup::deduplicate_messages(
            &current,
            self.config.dedup_threshold,
            self.config.shingle_size,
        );
        current = dedup_result;
        let after_dedup = estimate_tokens_batch(&current);
        if before_dedup > after_dedup {
            stages.push(StageSavings {
                name: "dedup".to_string(),
                tokens_saved: before_dedup - after_dedup,
            });
        }

        // Stage 2: Dictionary compression
        let before_dict = estimate_tokens_batch(&current);
        let texts: Vec<&str> = current.iter().map(|m| m.content.as_str()).collect();
        let codebook = dictionary::build_codebook(
            &texts,
            self.config.dict_min_freq,
            self.config.dict_max_entries,
        );
        if !codebook.is_empty() {
            for msg in &mut current {
                msg.content = dictionary::compress_text(&msg.content, &codebook);
            }
            let after_dict = estimate_tokens_batch(&current);
            if before_dict > after_dict {
                stages.push(StageSavings {
                    name: "dictionary".to_string(),
                    tokens_saved: before_dict - after_dict,
                });
            }
        }

        // Stage 3: Pattern compression
        let before_patterns = estimate_tokens_batch(&current);
        for msg in &mut current {
            msg.content = patterns::compress_patterns(&msg.content);
        }
        let after_patterns = estimate_tokens_batch(&current);
        if before_patterns > after_patterns {
            stages.push(StageSavings {
                name: "patterns".to_string(),
                tokens_saved: before_patterns - after_patterns,
            });
        }

        // Stage 4: Text optimization
        if self.config.text_optimize {
            let before_text = estimate_tokens_batch(&current);
            for msg in &mut current {
                msg.content =
                    text_optimizer::optimize_text(&msg.content, self.config.text_optimize_aggressive);
            }
            let after_text = estimate_tokens_batch(&current);
            if before_text > after_text {
                stages.push(StageSavings {
                    name: "text_optimize".to_string(),
                    tokens_saved: before_text - after_text,
                });
            }
        }

        let tokens_after = estimate_tokens_batch(&current);
        let ratio = if tokens_before > 0 {
            tokens_after as f64 / tokens_before as f64
        } else {
            1.0
        };

        CompressionResult {
            messages: current,
            tokens_before,
            tokens_after,
            ratio,
            stages,
        }
    }
}

impl Default for CompressorPipeline {
    fn default() -> Self {
        Self::new(CompressorConfig::default())
    }
}

/// Estimate tokens for a batch of messages.
fn estimate_tokens_batch(messages: &[ChatMessage]) -> usize {
    messages.iter().map(|m| estimate_tokens(&m.content)).sum()
}

/// Estimate tokens for a single text (CJK-aware heuristic).
pub fn estimate_tokens(text: &str) -> usize {
    let mut ascii_chars = 0usize;
    let mut cjk_chars = 0usize;

    for ch in text.chars() {
        if is_cjk(ch) {
            cjk_chars += 1;
        } else {
            ascii_chars += 1;
        }
    }

    // ASCII: ~4 chars/token; CJK: ~1.5 chars/token
    let ascii_tokens = ascii_chars / 4;
    let cjk_tokens = (cjk_chars as f64 / 1.5).ceil() as usize;

    ascii_tokens + cjk_tokens + 4 // +4 for message overhead
}

/// Check if a character is CJK.
fn is_cjk(ch: char) -> bool {
    matches!(ch,
        '\u{4e00}'..='\u{9fff}' |
        '\u{3400}'..='\u{4dbf}' |
        '\u{3000}'..='\u{303f}' |
        '\u{ff00}'..='\u{ffef}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        let tokens = estimate_tokens("Hello, world! This is a test.");
        assert!(tokens > 0);
        assert!(tokens < 50);
    }

    #[test]
    fn test_cjk_detection() {
        assert!(is_cjk('中'));
        assert!(is_cjk('日'));
        assert!(!is_cjk('A'));
        assert!(!is_cjk('1'));
    }

    #[test]
    fn test_pipeline_noop_on_empty() {
        let pipeline = CompressorPipeline::default();
        let result = pipeline.compress(&[]);
        assert!(result.messages.is_empty());
        assert_eq!(result.tokens_before, 0);
    }

    #[test]
    fn test_pipeline_preserves_messages() {
        let pipeline = CompressorPipeline::default();
        let msgs = vec![
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];
        let result = pipeline.compress(&msgs);
        assert_eq!(result.messages.len(), 2);
    }
}
