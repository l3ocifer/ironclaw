//! Dictionary compression with auto-learned codebook.
//!
//! Ported from claw-compactor's `dictionary.py`.
//!
//! Extracts frequently repeated n-gram phrases from text and replaces
//! them with short codes (`$AA`..`$ZZ`, up to 676 slots).

use std::collections::HashMap;

/// Build a codebook from a collection of texts.
///
/// Extracts n-gram phrases that appear at least `min_freq` times,
/// ranks by savings potential (freq * length), and generates codes.
///
/// Returns a map from code (e.g. "$AB") to the phrase it represents.
pub fn build_codebook(
    texts: &[&str],
    min_freq: usize,
    max_entries: usize,
) -> HashMap<String, String> {
    // Count n-gram frequencies across all texts
    let mut freq_map: HashMap<String, usize> = HashMap::new();

    for text in texts {
        let ngrams = extract_ngrams(text, 2, 5);
        for ngram in ngrams {
            *freq_map.entry(ngram).or_insert(0) += 1;
        }
    }

    // Filter by minimum frequency and minimum phrase length
    let mut candidates: Vec<(String, usize)> = freq_map
        .into_iter()
        .filter(|(phrase, count)| *count >= min_freq && phrase.len() >= 6)
        .collect();

    // Sort by savings potential: freq * phrase_length (descending)
    candidates.sort_by(|a, b| {
        let savings_a = a.1 * a.0.len();
        let savings_b = b.1 * b.0.len();
        savings_b.cmp(&savings_a)
    });

    // Generate codebook, avoiding overlapping phrases
    let mut codebook = HashMap::new();
    let mut used_phrases: Vec<String> = Vec::new();
    let mut code_gen = CodeGenerator::new_gen();

    for (phrase, _freq) in candidates {
        if codebook.len() >= max_entries {
            break;
        }

        // Skip if this phrase is a substring of an already-selected phrase
        let overlaps = used_phrases
            .iter()
            .any(|existing| existing.contains(&phrase) || phrase.contains(existing.as_str()));
        if overlaps {
            continue;
        }

        let code = code_gen.next();
        codebook.insert(code, phrase.clone());
        used_phrases.push(phrase);
    }

    codebook
}

/// Compress text using a codebook.
///
/// Replaces phrases with their codes. Existing `$` characters are escaped
/// to avoid collisions.
pub fn compress_text(text: &str, codebook: &HashMap<String, String>) -> String {
    if codebook.is_empty() {
        return text.to_string();
    }

    // Escape existing dollar signs
    let mut result = text.replace('$', "\x00DLR\x00");

    // Sort phrases by length descending to avoid partial matches
    let mut entries: Vec<(&String, &String)> = codebook.iter().collect();
    entries.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (code, phrase) in entries {
        result = result.replace(phrase.as_str(), code);
    }

    result
}

/// Decompress text by expanding codes back to phrases.
pub fn decompress_text(text: &str, codebook: &HashMap<String, String>) -> String {
    let mut result = text.to_string();

    // Sort codes by length descending ($AAA before $AA)
    let mut entries: Vec<(&String, &String)> = codebook.iter().collect();
    entries.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (code, phrase) in entries {
        result = result.replace(code.as_str(), phrase);
    }

    // Unescape dollar signs
    result = result.replace("\x00DLR\x00", "$");

    result
}

/// Extract word n-grams from text.
fn extract_ngrams(text: &str, min_n: usize, max_n: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut ngrams = Vec::new();

    for n in min_n..=max_n {
        if words.len() < n {
            continue;
        }
        for window in words.windows(n) {
            let phrase = window.join(" ");
            if phrase.len() >= 6 {
                ngrams.push(phrase);
            }
        }
    }

    ngrams
}

/// Generates codes: $AA, $AB, ..., $AZ, $BA, ..., $ZZ (676 codes).
struct CodeGenerator {
    index: usize,
}

impl CodeGenerator {
    fn new_gen() -> Self {
        Self { index: 0 }
    }

    fn next(&mut self) -> String {
        let first = (b'A' + (self.index / 26) as u8) as char;
        let second = (b'A' + (self.index % 26) as u8) as char;
        self.index += 1;

        if self.index <= 676 {
            format!("${}{}", first, second)
        } else {
            // Overflow to 3-letter codes
            let i = self.index - 676;
            let a = (b'A' + (i / 676) as u8) as char;
            let b_ch = (b'A' + ((i / 26) % 26) as u8) as char;
            let c = (b'A' + (i % 26) as u8) as char;
            format!("${}{}{}", a, b_ch, c)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_generator() {
        let mut code_gen = CodeGenerator::new_gen();
        assert_eq!(code_gen.next(), "$AA");
        assert_eq!(code_gen.next(), "$AB");
        assert_eq!(code_gen.next(), "$AC");
    }

    #[test]
    fn test_extract_ngrams() {
        let ngrams = extract_ngrams("the quick brown fox jumps over", 2, 3);
        assert!(!ngrams.is_empty());
        // 2-grams: "the quick", "quick brown", etc.
        // 3-grams: "the quick brown", etc.
        assert!(ngrams.iter().any(|n| n == "the quick brown"));
    }

    #[test]
    fn test_build_codebook() {
        let texts = vec![
            "the quick brown fox the quick brown fox the quick brown fox",
            "the quick brown dog the quick brown dog the quick brown dog",
        ];
        let codebook = build_codebook(&texts, 2, 10);
        // "the quick brown" appears 6 times across both, should be in codebook
        assert!(codebook.values().any(|v| v.contains("quick brown")));
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let mut codebook = HashMap::new();
        codebook.insert("$AA".to_string(), "hello world".to_string());

        let original = "say hello world to hello world";
        let compressed = compress_text(original, &codebook);
        assert!(compressed.contains("$AA"));
        assert!(!compressed.contains("hello world"));

        let decompressed = decompress_text(&compressed, &codebook);
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_dollar_sign_escaping() {
        let codebook = HashMap::new();
        let text = "price is $100";
        let compressed = compress_text(text, &codebook);
        let decompressed = decompress_text(&compressed, &codebook);
        assert_eq!(decompressed, text);
    }
}
