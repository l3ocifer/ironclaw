//! Skill discovery and loading.
//!
//! Loads skills from multiple directories with defined precedence.
//! Later sources override earlier ones (by skill name).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::{debug, info, warn};

use super::frontmatter::{parse_frontmatter, resolve_metadata};
use super::{Skill, SkillEntry};

/// Resolve the bundled skills directory.
///
/// Search order:
/// 1. `IRONCLAW_SKILLS_DIR` env var (explicit override)
/// 2. `<exe_dir>/skills/` (production: skills shipped alongside binary)
/// 3. `<exe_dir>/../skills/` (dev: binary in `target/debug/`, skills at repo root)
/// 4. `./skills/` (fallback: current working directory)
///
/// Returns `None` if no candidate directory exists.
pub fn resolve_bundled_skills_dir() -> Option<PathBuf> {
    // 1. Env var override
    if let Ok(dir) = std::env::var("IRONCLAW_SKILLS_DIR") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            info!("Using bundled skills from IRONCLAW_SKILLS_DIR: {}", path.display());
            return Some(path);
        }
    }

    // 2. Relative to executable
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        // 2a. <exe_dir>/skills/
        let alongside = exe_dir.join("skills");
        if alongside.is_dir() {
            info!("Using bundled skills from exe dir: {}", alongside.display());
            return Some(alongside);
        }

        // 2b. <exe_dir>/../skills/ (dev layout: target/debug/../../../skills)
        // Walk up from exe looking for skills/ (handles target/debug, target/release, etc.)
        let mut search = exe_dir.to_path_buf();
        for _ in 0..5 {
            let candidate = search.join("skills");
            if candidate.is_dir() && is_skills_directory(&candidate) {
                info!("Using bundled skills from ancestor: {}", candidate.display());
                return Some(candidate);
            }
            if !search.pop() {
                break;
            }
        }
    }

    // 3. CWD fallback
    let cwd = PathBuf::from("skills");
    if cwd.is_dir() && is_skills_directory(&cwd) {
        info!("Using bundled skills from CWD: {}", cwd.display());
        return Some(std::fs::canonicalize(&cwd).unwrap_or(cwd));
    }

    None
}

/// Quick heuristic: does this directory look like a skills directory?
fn is_skills_directory(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let skill_md = entry.path().join("SKILL.md");
                if skill_md.exists() {
                    return true;
                }
            }
        }
    }
    false
}

/// Options for loading skills from all configured locations.
#[derive(Debug, Clone, Default)]
pub struct SkillLoadOptions {
    /// Bundled skills directory (shipped with binary).
    pub bundled_dir: Option<PathBuf>,
    /// Managed skills directory (default: `~/.ironclaw/skills/`).
    pub managed_dir: Option<PathBuf>,
    /// Workspace skills directory (e.g. `<workspace>/skills/`).
    pub workspace_dir: Option<PathBuf>,
    /// Additional skill directories configured by the user.
    pub extra_dirs: Vec<PathBuf>,
    /// Whether to also scan `~/.claude/skills/` for compatibility.
    pub include_claude_skills: bool,
    /// Whether to also scan `~/.cursor/skills/` for compatibility.
    pub include_cursor_skills: bool,
    /// Per-skill configuration (enabled/disabled, env overrides).
    pub skill_config: HashMap<String, SkillConfig>,
    /// Allowlist for bundled skills. If non-empty, only listed bundled skills are loaded.
    pub bundled_allowlist: Vec<String>,
    /// Optional skill name filter. If non-empty, only these skills are included.
    pub skill_filter: Option<Vec<String>>,
}

/// Per-skill configuration.
#[derive(Debug, Clone, Default)]
pub struct SkillConfig {
    /// Explicitly enable/disable this skill.
    pub enabled: Option<bool>,
    /// API key override for the skill's primary env var.
    pub api_key: Option<String>,
    /// Additional environment variable overrides.
    pub env: HashMap<String, String>,
}

/// Load skills from a single directory.
///
/// Discovery rules:
/// - Direct `.md` children in the root directory
/// - Recursive `SKILL.md` files under subdirectories
pub fn load_skills_from_dir(dir: &Path, source: &str) -> Vec<Skill> {
    load_skills_from_dir_internal(dir, source, true)
}

fn load_skills_from_dir_internal(dir: &Path, source: &str, include_root_files: bool) -> Vec<Skill> {
    let mut skills = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return skills;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("Failed to read skills directory {}: {}", dir.display(), e);
            return skills;
        }
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip hidden files/dirs
        if name_str.starts_with('.') {
            continue;
        }
        // Skip node_modules
        if name_str == "node_modules" || name_str == "target" {
            continue;
        }

        let path = entry.path();

        // Resolve symlinks for type checking, but use the original path
        let file_type = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if file_type.is_dir() {
            let sub_skills = load_skills_from_dir_internal(&path, source, false);
            skills.extend(sub_skills);
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let is_root_md = include_root_files && name_str.ends_with(".md");
        let is_skill_md = !include_root_files && name_str == "SKILL.md";

        if !is_root_md && !is_skill_md {
            continue;
        }

        if let Some(skill) = load_skill_from_file(&path, source) {
            skills.push(skill);
        }
    }

    skills
}

/// Load a single skill from a SKILL.md file.
fn load_skill_from_file(file_path: &Path, source: &str) -> Option<Skill> {
    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read skill file {}: {}", file_path.display(), e);
            return None;
        }
    };

    let frontmatter = parse_frontmatter(&content);

    let skill_dir = file_path.parent()?;
    let parent_dir_name = skill_dir.file_name()?.to_string_lossy().to_string();

    // Use name from frontmatter, or fall back to parent directory name
    let name = frontmatter
        .get("name")
        .map(|s| s.to_string())
        .unwrap_or_else(|| parent_dir_name.clone());

    // Description is required
    let description = match frontmatter.get("description") {
        Some(d) if !d.trim().is_empty() => d.clone(),
        _ => {
            debug!(
                "Skipping skill at {} — no description in frontmatter",
                file_path.display()
            );
            return None;
        }
    };

    // Validate name
    if name.len() > 64 {
        warn!(
            "Skill name '{}' exceeds 64 characters at {}",
            name,
            file_path.display()
        );
    }

    let disable_model_invocation = frontmatter
        .get("disable-model-invocation")
        .map(|v| v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    Some(Skill {
        name,
        description,
        file_path: file_path.to_path_buf(),
        base_dir: skill_dir.to_path_buf(),
        source: source.to_string(),
        disable_model_invocation,
    })
}

/// Load skills from all configured directories.
///
/// Precedence (later wins on name collision):
/// 1. Extra dirs
/// 2. Bundled
/// 3. Claude skills (`~/.claude/skills/`)
/// 4. Cursor skills (`~/.cursor/skills/`)
/// 5. Managed (`~/.ironclaw/skills/`)
/// 6. Workspace (`<workspace>/skills/`)
pub fn load_skills(opts: &SkillLoadOptions) -> Vec<SkillEntry> {
    let mut merged: HashMap<String, Skill> = HashMap::new();

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    // 1. Extra dirs (lowest precedence)
    for dir in &opts.extra_dirs {
        for skill in load_skills_from_dir(dir, "ironclaw-extra") {
            merged.insert(skill.name.clone(), skill);
        }
    }

    // 2. Bundled
    if let Some(ref dir) = opts.bundled_dir {
        for skill in load_skills_from_dir(dir, "ironclaw-bundled") {
            // Apply bundled allowlist if configured
            if !opts.bundled_allowlist.is_empty()
                && !opts.bundled_allowlist.contains(&skill.name)
            {
                continue;
            }
            merged.insert(skill.name.clone(), skill);
        }
    }

    // 3. Claude skills (Anthropic ecosystem compatibility)
    if opts.include_claude_skills {
        let claude_dir = home.join(".claude").join("skills");
        for skill in load_skills_from_dir(&claude_dir, "claude") {
            merged.insert(skill.name.clone(), skill);
        }
    }

    // 4. Cursor skills (IDE compatibility)
    if opts.include_cursor_skills {
        let cursor_dir = home.join(".cursor").join("skills");
        for skill in load_skills_from_dir(&cursor_dir, "cursor") {
            merged.insert(skill.name.clone(), skill);
        }
    }

    // 5. Managed
    let managed_dir = opts
        .managed_dir
        .clone()
        .unwrap_or_else(|| home.join(".ironclaw").join("skills"));
    for skill in load_skills_from_dir(&managed_dir, "ironclaw-managed") {
        merged.insert(skill.name.clone(), skill);
    }

    // 6. Workspace (highest precedence)
    if let Some(ref dir) = opts.workspace_dir {
        for skill in load_skills_from_dir(dir, "ironclaw-workspace") {
            merged.insert(skill.name.clone(), skill);
        }
    }

    // Build SkillEntry with parsed metadata
    let mut entries: Vec<SkillEntry> = merged
        .into_values()
        .map(|skill| {
            let content = std::fs::read_to_string(&skill.file_path).unwrap_or_default();
            let frontmatter = parse_frontmatter(&content);
            let metadata = resolve_metadata(&frontmatter);
            SkillEntry {
                skill,
                frontmatter,
                metadata,
            }
        })
        .collect();

    // Sort by name for deterministic ordering
    entries.sort_by(|a, b| a.skill.name.cmp(&b.skill.name));

    // Apply skill filter if specified
    if let Some(ref filter) = opts.skill_filter {
        let normalized: Vec<String> = filter.iter().map(|s| s.trim().to_string()).collect();
        if !normalized.is_empty() {
            entries.retain(|e| normalized.contains(&e.skill.name));
        }
    }

    let count = entries.len();
    if count > 0 {
        debug!(
            "Loaded {} skills: {}",
            count,
            entries
                .iter()
                .map(|e| e.skill.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_skill(dir: &Path, name: &str, description: &str) {
        let skill_dir = dir.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        let content = format!(
            "---\nname: {}\ndescription: {}\n---\n# {}\nInstructions here.\n",
            name, description, name
        );
        fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn test_load_skills_from_dir() {
        let tmp = tempfile::tempdir().unwrap();
        create_test_skill(tmp.path(), "weather", "Get weather forecasts.");
        create_test_skill(tmp.path(), "github", "GitHub CLI integration.");

        let skills = load_skills_from_dir(tmp.path(), "test");
        assert_eq!(skills.len(), 2);

        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"weather"));
        assert!(names.contains(&"github"));
    }

    #[test]
    fn test_load_skills_from_dir_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let skills = load_skills_from_dir(tmp.path(), "test");
        assert!(skills.is_empty());
    }

    #[test]
    fn test_load_skills_from_dir_nonexistent() {
        let skills = load_skills_from_dir(Path::new("/nonexistent/path"), "test");
        assert!(skills.is_empty());
    }

    #[test]
    fn test_skill_precedence() {
        let low = tempfile::tempdir().unwrap();
        let high = tempfile::tempdir().unwrap();

        create_test_skill(low.path(), "weather", "Low priority weather.");
        create_test_skill(high.path(), "weather", "High priority weather.");

        let opts = SkillLoadOptions {
            extra_dirs: vec![low.path().to_path_buf()],
            managed_dir: Some(high.path().to_path_buf()),
            include_claude_skills: false,
            include_cursor_skills: false,
            ..Default::default()
        };

        let entries = load_skills(&opts);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].skill.description, "High priority weather.");
        assert_eq!(entries[0].skill.source, "ironclaw-managed");
    }
}
