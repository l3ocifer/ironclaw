//! Semantic merge for multi-agent workspace files.
//!
//! Uses [`weave_core`] for entity-level 3-way merge instead of line-level diff.
//! This is critical when multiple agents (e.g., Frack + Frick) edit the same
//! workspace files concurrently — entity-level merge understands function
//! boundaries, markdown sections, YAML keys, etc., producing far fewer
//! false conflicts than line-based merge.
//!
//! # Supported file types
//!
//! weave-core delegates to [`sem_core`] for entity extraction via tree-sitter:
//! TypeScript, JavaScript, Python, Go, Rust, JSON, YAML, TOML, Markdown, Java, C.
//!
//! # Usage
//!
//! ```ignore
//! use ironclaw::workspace::merge::semantic_merge;
//!
//! let result = semantic_merge(base, ours, theirs, "MEMORY.md");
//! if result.conflicts.is_empty() {
//!     // Clean merge
//!     save(result.content);
//! } else {
//!     // Conflicts need resolution
//!     for conflict in &result.conflicts {
//!         log::warn!("Conflict in entity: {:?}", conflict);
//!     }
//! }
//! ```

use tracing::{debug, warn};
use weave_core::MergeResult;

/// Perform a semantic 3-way merge.
///
/// - `base`: common ancestor content
/// - `ours`: our version (local agent's edits)
/// - `theirs`: their version (remote agent's edits)
/// - `file_path`: used to determine the parser (e.g., `.md`, `.rs`, `.json`)
///
/// Returns a [`MergeResult`] with the merged content, any conflicts, warnings,
/// and merge statistics.
pub fn semantic_merge(base: &str, ours: &str, theirs: &str, file_path: &str) -> MergeResult {
    debug!(
        file_path,
        base_len = base.len(),
        ours_len = ours.len(),
        theirs_len = theirs.len(),
        "Performing semantic merge"
    );

    let result = weave_core::entity_merge(base, ours, theirs, file_path);

    if !result.conflicts.is_empty() {
        warn!(
            file_path,
            conflict_count = result.conflicts.len(),
            "Semantic merge produced conflicts"
        );
    }

    if !result.warnings.is_empty() {
        debug!(
            file_path,
            warning_count = result.warnings.len(),
            "Semantic merge warnings"
        );
    }

    debug!(
        file_path,
        entities_both_changed = result.stats.entities_both_changed_merged,
        entities_conflicted = result.stats.entities_conflicted,
        "Semantic merge complete"
    );

    result
}

/// Try semantic merge, falling back to "ours wins" on conflict.
///
/// Useful for automated background merges where we want to prefer the local
/// agent's changes but still incorporate non-conflicting remote changes.
pub fn merge_prefer_ours(base: &str, ours: &str, theirs: &str, file_path: &str) -> String {
    let result = semantic_merge(base, ours, theirs, file_path);

    if result.conflicts.is_empty() {
        result.content
    } else {
        warn!(
            file_path,
            conflicts = result.conflicts.len(),
            "Conflicts detected, preferring ours"
        );
        // Return our version when conflicts exist
        ours.to_string()
    }
}

/// Merge workspace memory files with conflict markers.
///
/// Similar to git merge — inserts `<<<<<<<`, `=======`, `>>>>>>>` markers
/// for conflicts that need manual (or LLM-assisted) resolution.
pub fn merge_with_markers(
    base: &str,
    ours: &str,
    theirs: &str,
    file_path: &str,
    our_label: &str,
    their_label: &str,
) -> MergeWithMarkersResult {
    let result = semantic_merge(base, ours, theirs, file_path);

    if result.conflicts.is_empty() {
        return MergeWithMarkersResult {
            content: result.content,
            had_conflicts: false,
            conflict_count: 0,
        };
    }

    // Build content with conflict markers for unresolved conflicts
    let mut output = result.content.clone();
    for conflict in &result.conflicts {
        let ours_text = conflict.ours_content.as_deref().unwrap_or("");
        let theirs_text = conflict.theirs_content.as_deref().unwrap_or("");
        let marker = format!(
            "<<<<<<< {our_label}\n{ours_text}\n=======\n{theirs_text}\n>>>>>>> {their_label}",
        );
        // Replace the conflict region in the output (best-effort)
        if !ours_text.is_empty() && let Some(pos) = output.find(ours_text) {
            output.replace_range(pos..pos + ours_text.len(), &marker);
        } else {
            warn!(
                file_path,
                "Could not place conflict marker — ours_text not found in merged output"
            );
        }
    }

    MergeWithMarkersResult {
        content: output,
        had_conflicts: true,
        conflict_count: result.conflicts.len(),
    }
}

/// Result of a merge-with-markers operation.
#[derive(Debug)]
pub struct MergeWithMarkersResult {
    /// The merged content (may include conflict markers).
    pub content: String,
    /// Whether any conflicts were found.
    pub had_conflicts: bool,
    /// Number of conflicts.
    pub conflict_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_merge_no_conflict() {
        let base = "# Memory\n\n## Notes\n\nOriginal note.\n";
        let ours = "# Memory\n\n## Notes\n\nOriginal note.\n\n## Tasks\n\n- Buy milk\n";
        let theirs = "# Memory\n\n## Notes\n\nOriginal note.\n\nUpdated by Frick.\n";

        let result = semantic_merge(base, ours, theirs, "MEMORY.md");
        // Both changes are in different sections/locations, should merge cleanly
        assert!(
            result.conflicts.is_empty(),
            "Expected clean merge, got {} conflicts",
            result.conflicts.len()
        );
    }

    #[test]
    fn test_merge_prefer_ours_on_conflict() {
        let base = "line1\nline2\nline3\n";
        let ours = "line1\nOUR CHANGE\nline3\n";
        let theirs = "line1\nTHEIR CHANGE\nline3\n";

        let merged = merge_prefer_ours(base, ours, theirs, "test.txt");
        // On conflict, should prefer ours
        assert!(
            merged.contains("OUR CHANGE"),
            "Expected 'OUR CHANGE' in merged output, got: {merged}"
        );
    }
}
