//! SKILL.md frontmatter parsing.
//!
//! Parses YAML frontmatter from SKILL.md files. Frontmatter is delimited
//! by `---` lines at the top of the file.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Raw frontmatter key-value pairs from a SKILL.md file.
pub type SkillFrontmatter = HashMap<String, String>;

/// IronClaw-specific skill metadata (parsed from the `metadata` frontmatter field).
///
/// Compatible with OpenClaw's `openclaw` metadata key.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// If true, always include this skill regardless of eligibility checks.
    #[serde(default)]
    pub always: Option<bool>,
    /// Override skill key for config lookups.
    #[serde(default)]
    pub skill_key: Option<String>,
    /// Primary environment variable (e.g. API key) required by the skill.
    #[serde(default)]
    pub primary_env: Option<String>,
    /// Emoji for display purposes.
    #[serde(default)]
    pub emoji: Option<String>,
    /// Homepage URL.
    #[serde(default)]
    pub homepage: Option<String>,
    /// OS restrictions (e.g. ["darwin", "linux"]).
    #[serde(default)]
    pub os: Vec<String>,
    /// Required system dependencies.
    #[serde(default)]
    pub requires: Option<SkillRequirements>,
}

/// Requirements that must be met for a skill to be eligible.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillRequirements {
    /// All of these binaries must be on PATH.
    #[serde(default)]
    pub bins: Vec<String>,
    /// At least one of these binaries must be on PATH.
    #[serde(default)]
    pub any_bins: Vec<String>,
    /// All of these environment variables must be set.
    #[serde(default)]
    pub env: Vec<String>,
    /// All of these config paths must be truthy.
    #[serde(default)]
    pub config: Vec<String>,
}

/// Parse YAML frontmatter from a SKILL.md file.
///
/// Returns an empty map if no frontmatter is found or if parsing fails.
/// Frontmatter is delimited by `---` lines:
///
/// ```text
/// ---
/// name: my-skill
/// description: Does something useful
/// ---
/// # My Skill
/// ...instructions...
/// ```
pub fn parse_frontmatter(content: &str) -> SkillFrontmatter {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return SkillFrontmatter::new();
    }

    // Find the closing ---
    let after_first = &trimmed[3..];
    let rest = after_first.trim_start_matches(['\r', '\n']);

    let closing = rest.find("\n---");
    let Some(closing_pos) = closing else {
        return SkillFrontmatter::new();
    };

    let yaml_block = &rest[..closing_pos];
    parse_yaml_block(yaml_block)
}

/// Parse a simple YAML block into key-value pairs.
///
/// Handles single-line values and multi-line values (indented continuation or JSON blocks).
fn parse_yaml_block(yaml: &str) -> SkillFrontmatter {
    let mut map = SkillFrontmatter::new();
    let mut current_key: Option<String> = None;
    let mut current_value = String::new();

    for line in yaml.lines() {
        // Check if this is a top-level key (no leading whitespace, contains `:`)
        if !line.starts_with(' ') && !line.starts_with('\t') {
            // Flush previous key-value
            if let Some(ref key) = current_key {
                let val = current_value.trim().to_string();
                if !val.is_empty() {
                    map.insert(key.clone(), strip_yaml_quotes(&val));
                }
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                current_key = Some(key);
                current_value = value;
            } else {
                current_key = None;
                current_value.clear();
            }
        } else if current_key.is_some() {
            // Continuation line — append to current value
            current_value.push('\n');
            current_value.push_str(line);
        }
    }

    // Flush last key-value
    if let Some(ref key) = current_key {
        let val = current_value.trim().to_string();
        if !val.is_empty() {
            map.insert(key.clone(), strip_yaml_quotes(&val));
        }
    }

    map
}

/// Strip trailing commas before closing braces/brackets (JSON5 → JSON).
///
/// Handles patterns like `{ "a": 1, }` → `{ "a": 1 }`.
fn strip_trailing_commas(s: &str) -> String {
    let re = regex::Regex::new(r",(\s*[}\]])").unwrap();
    re.replace_all(s, "$1").to_string()
}

/// Strip surrounding quotes from a YAML string value.
fn strip_yaml_quotes(s: &str) -> String {
    let trimmed = s.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Resolve IronClaw-specific metadata from the `metadata` frontmatter field.
///
/// The metadata field contains a JSON/JSON5 object with an `openclaw` or
/// `ironclaw` key holding the actual metadata.
pub fn resolve_metadata(frontmatter: &SkillFrontmatter) -> Option<SkillMetadata> {
    let raw = frontmatter.get("metadata")?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // The metadata field may contain JSON5 (trailing commas, etc.).
    // Strip trailing commas before closing braces/brackets for serde_json compatibility.
    let cleaned = strip_trailing_commas(trimmed);

    // Try to parse as JSON. The metadata field typically looks like:
    // metadata: { "openclaw": { ... } }
    let parsed: serde_json::Value = serde_json::from_str(&cleaned).ok()?;
    let obj = parsed.as_object()?;

    // Look for ironclaw or openclaw key (ironclaw takes precedence)
    let metadata_obj = obj
        .get("ironclaw")
        .or_else(|| obj.get("openclaw"))
        .and_then(|v| v.as_object())?;

    let metadata_value = serde_json::Value::Object(metadata_obj.clone());

    // Deserialize with custom handling for field name differences
    let mut meta = SkillMetadata::default();

    if let Some(always) = metadata_value.get("always").and_then(|v| v.as_bool()) {
        meta.always = Some(always);
    }
    if let Some(key) = metadata_value
        .get("skillKey")
        .and_then(|v| v.as_str())
    {
        meta.skill_key = Some(key.to_string());
    }
    if let Some(env) = metadata_value
        .get("primaryEnv")
        .and_then(|v| v.as_str())
    {
        meta.primary_env = Some(env.to_string());
    }
    if let Some(emoji) = metadata_value.get("emoji").and_then(|v| v.as_str()) {
        meta.emoji = Some(emoji.to_string());
    }
    if let Some(homepage) = metadata_value.get("homepage").and_then(|v| v.as_str()) {
        meta.homepage = Some(homepage.to_string());
    }
    if let Some(os_arr) = metadata_value.get("os").and_then(|v| v.as_array()) {
        meta.os = os_arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(requires) = metadata_value.get("requires").and_then(|v| v.as_object()) {
        let mut reqs = SkillRequirements::default();
        if let Some(bins) = requires.get("bins").and_then(|v| v.as_array()) {
            reqs.bins = bins
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        if let Some(any_bins) = requires.get("anyBins").and_then(|v| v.as_array()) {
            reqs.any_bins = any_bins
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        if let Some(env) = requires.get("env").and_then(|v| v.as_array()) {
            reqs.env = env
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        if let Some(config) = requires.get("config").and_then(|v| v.as_array()) {
            reqs.config = config
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        meta.requires = Some(reqs);
    }

    Some(meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_basic() {
        let content = r#"---
name: weather
description: Get current weather and forecasts (no API key required).
---
# Weather
Instructions here.
"#;
        let fm = parse_frontmatter(content);
        assert_eq!(fm.get("name").unwrap(), "weather");
        assert_eq!(
            fm.get("description").unwrap(),
            "Get current weather and forecasts (no API key required)."
        );
    }

    #[test]
    fn test_parse_frontmatter_quoted() {
        let content = r#"---
name: github
description: "Interact with GitHub using the `gh` CLI."
---
"#;
        let fm = parse_frontmatter(content);
        assert_eq!(fm.get("name").unwrap(), "github");
        assert_eq!(
            fm.get("description").unwrap(),
            "Interact with GitHub using the `gh` CLI."
        );
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let content = "# Just a markdown file\nNo frontmatter here.";
        let fm = parse_frontmatter(content);
        assert!(fm.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_with_metadata() {
        let content = r#"---
name: weather
description: Get weather.
metadata: { "openclaw": { "emoji": "🌤️", "requires": { "bins": ["curl"] } } }
---
"#;
        let fm = parse_frontmatter(content);
        let meta = resolve_metadata(&fm).unwrap();
        assert_eq!(meta.emoji.as_deref(), Some("🌤️"));
        let reqs = meta.requires.unwrap();
        assert_eq!(reqs.bins, vec!["curl"]);
    }

    #[test]
    fn test_parse_frontmatter_multiline_metadata() {
        let content = r#"---
name: github
description: "GitHub CLI skill"
metadata:
  {
    "openclaw":
      {
        "emoji": "🐙",
        "requires": { "bins": ["gh"] },
      },
  }
---
"#;
        let fm = parse_frontmatter(content);
        assert_eq!(fm.get("name").unwrap(), "github");
        let meta = resolve_metadata(&fm);
        assert!(meta.is_some());
        let meta = meta.unwrap();
        assert_eq!(meta.emoji.as_deref(), Some("🐙"));
    }
}
