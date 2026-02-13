//! Agent Skills infrastructure (ported from OpenClaw).
//!
//! Skills are modular capability bundles — a directory containing a `SKILL.md`
//! file with YAML frontmatter (name, description, metadata) plus optional
//! scripts and reference materials. They teach the agent **how** and **when**
//! to use tools for a specific domain (e.g. GitHub, Slack, weather).
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │                System Prompt (always)                   │
//! │  <available_skills>                                    │
//! │    <skill>                                             │
//! │      <name>github</name>                               │
//! │      <description>Interact with GitHub using gh CLI    │
//! │      </description>                                    │
//! │      <location>/path/to/github/SKILL.md</location>     │
//! │    </skill>                                             │
//! │    ...                                                  │
//! │  </available_skills>                                   │
//! └────────────────────────────────────────────────────────┘
//! ```
//!
//! Only name + description are loaded into context (~100 tokens each).
//! The full SKILL.md is loaded on-demand via the agent's read/shell tool
//! when a task matches the skill's description (progressive disclosure).
//!
//! # Skill Sources (precedence: later wins)
//!
//! 1. **Extra dirs** — user-configured additional directories
//! 2. **Bundled** — shipped with IronClaw binary
//! 3. **Claude** — `~/.claude/skills/` (Anthropic ecosystem compatibility)
//! 4. **Cursor** — `~/.cursor/skills/` (Cursor IDE compatibility)
//! 5. **Managed** — `~/.ironclaw/skills/` (user-installed)
//! 6. **Workspace** — `<workspace>/skills/` (project-specific)

mod eligibility;
mod frontmatter;
mod loader;
mod prompt;

pub use eligibility::should_include_skill;
pub use frontmatter::{parse_frontmatter, SkillFrontmatter, SkillMetadata};
pub use loader::{
    load_skills, load_skills_from_dir, resolve_bundled_skills_dir, SkillConfig, SkillLoadOptions,
};
pub use prompt::format_skills_for_prompt;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A loaded skill — metadata parsed from SKILL.md frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Unique name (lowercase, hyphens, digits only). Must match parent directory name.
    pub name: String,
    /// Human-readable description of what the skill does and when to use it.
    pub description: String,
    /// Absolute path to the SKILL.md file.
    pub file_path: PathBuf,
    /// Absolute path to the skill's base directory (parent of SKILL.md).
    pub base_dir: PathBuf,
    /// Source identifier (e.g. "ironclaw-bundled", "ironclaw-managed", "claude", "cursor").
    pub source: String,
    /// If true, the skill is excluded from the system prompt and can only be invoked explicitly.
    pub disable_model_invocation: bool,
}

/// A skill entry with parsed frontmatter and resolved metadata.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// The loaded skill.
    pub skill: Skill,
    /// Raw frontmatter key-value pairs.
    pub frontmatter: SkillFrontmatter,
    /// Parsed IronClaw-specific metadata (requirements, install specs, etc.).
    pub metadata: Option<SkillMetadata>,
}

/// A snapshot of the skills state for a given agent run.
#[derive(Debug, Clone)]
pub struct SkillSnapshot {
    /// Formatted prompt text to inject into the system prompt.
    pub prompt: String,
    /// Summary of included skills (name + optional primary env).
    pub skills: Vec<SkillSummary>,
}

/// Summary of a single skill in the snapshot.
#[derive(Debug, Clone)]
pub struct SkillSummary {
    /// Skill name.
    pub name: String,
    /// Primary environment variable required by the skill (e.g. GITHUB_TOKEN).
    pub primary_env: Option<String>,
}

/// Build a complete skill snapshot for the current agent run.
///
/// Loads skills from all configured directories, filters by eligibility,
/// and formats the prompt text. This is the main entry point for the
/// agent loop.
pub fn build_skill_snapshot(opts: &SkillLoadOptions) -> SkillSnapshot {
    let entries = load_skills(opts);
    let eligible: Vec<&SkillEntry> = entries
        .iter()
        .filter(|e| should_include_skill(e, opts))
        .collect();

    let prompt_entries: Vec<&Skill> = eligible
        .iter()
        .filter(|e| !e.skill.disable_model_invocation)
        .map(|e| &e.skill)
        .collect();

    let prompt = format_skills_for_prompt(&prompt_entries);

    let skills = eligible
        .iter()
        .map(|e| SkillSummary {
            name: e.skill.name.clone(),
            primary_env: e.metadata.as_ref().and_then(|m| m.primary_env.clone()),
        })
        .collect();

    SkillSnapshot { prompt, skills }
}
