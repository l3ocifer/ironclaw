//! Skill eligibility checking.
//!
//! Determines whether a skill should be included based on its requirements
//! (required binaries, environment variables, OS, config) and user settings.

use std::env;
use std::path::Path;

use tracing::debug;

use super::loader::SkillLoadOptions;
use super::SkillEntry;

/// Check whether a skill should be included in the current agent run.
///
/// A skill is excluded if:
/// 1. It is explicitly disabled in per-skill config
/// 2. Its OS requirements don't match the current platform
/// 3. Its required binaries are not on PATH
/// 4. Its required environment variables are not set
///
/// A skill with `always: true` in metadata skips requirements checks
/// (but can still be disabled via config).
pub fn should_include_skill(entry: &SkillEntry, opts: &SkillLoadOptions) -> bool {
    let skill_key = entry
        .metadata
        .as_ref()
        .and_then(|m| m.skill_key.as_deref())
        .unwrap_or(&entry.skill.name);

    // Check per-skill config — explicit disable
    if let Some(config) = opts.skill_config.get(skill_key) {
        if config.enabled == Some(false) {
            debug!("Skill '{}' disabled by config", entry.skill.name);
            return false;
        }
    }

    // Check OS restriction
    if let Some(ref meta) = entry.metadata {
        if !meta.os.is_empty() {
            let current_os = current_platform();
            if !meta.os.iter().any(|os| os == current_os) {
                debug!(
                    "Skill '{}' excluded — OS '{}' not in {:?}",
                    entry.skill.name, current_os, meta.os
                );
                return false;
            }
        }

        // If `always: true`, skip remaining requirement checks
        if meta.always == Some(true) {
            return true;
        }

        // Check required binaries
        if let Some(ref requires) = meta.requires {
            // All required bins must be present
            for bin in &requires.bins {
                if !has_binary(bin) {
                    debug!(
                        "Skill '{}' excluded — required binary '{}' not found",
                        entry.skill.name, bin
                    );
                    return false;
                }
            }

            // At least one of any_bins must be present
            if !requires.any_bins.is_empty()
                && !requires.any_bins.iter().any(|bin| has_binary(bin))
            {
                debug!(
                    "Skill '{}' excluded — none of {:?} found",
                    entry.skill.name, requires.any_bins
                );
                return false;
            }

            // All required env vars must be set
            for env_name in &requires.env {
                if env::var(env_name).is_ok() {
                    continue;
                }
                // Check per-skill env overrides
                if let Some(config) = opts.skill_config.get(skill_key) {
                    if config.env.contains_key(env_name) {
                        continue;
                    }
                    // Check apiKey as fallback for primaryEnv
                    if config.api_key.is_some()
                        && meta.primary_env.as_deref() == Some(env_name)
                    {
                        continue;
                    }
                }
                debug!(
                    "Skill '{}' excluded — required env '{}' not set",
                    entry.skill.name, env_name
                );
                return false;
            }
        }
    }

    true
}

/// Check if a binary exists on PATH.
fn has_binary(bin: &str) -> bool {
    let path_env = env::var("PATH").unwrap_or_default();
    let separator = if cfg!(windows) { ';' } else { ':' };

    for dir in path_env.split(separator) {
        let candidate = Path::new(dir).join(bin);
        if candidate.exists() {
            // On Unix, also check if executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = candidate.metadata() {
                    if meta.permissions().mode() & 0o111 != 0 {
                        return true;
                    }
                }
            }
            #[cfg(not(unix))]
            {
                return true;
            }
        }
    }
    false
}

/// Get the current platform identifier (matching Rust's `std::env::consts::OS`).
fn current_platform() -> &'static str {
    std::env::consts::OS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::frontmatter::{SkillMetadata, SkillRequirements};
    use crate::skills::{Skill, SkillEntry};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_entry(name: &str, metadata: Option<SkillMetadata>) -> SkillEntry {
        SkillEntry {
            skill: Skill {
                name: name.to_string(),
                description: "test".to_string(),
                file_path: PathBuf::from("/tmp/test/SKILL.md"),
                base_dir: PathBuf::from("/tmp/test"),
                source: "test".to_string(),
                disable_model_invocation: false,
            },
            frontmatter: HashMap::new(),
            metadata,
        }
    }

    #[test]
    fn test_no_requirements_passes() {
        let entry = make_entry("simple", None);
        let opts = SkillLoadOptions::default();
        assert!(should_include_skill(&entry, &opts));
    }

    #[test]
    fn test_disabled_by_config() {
        let entry = make_entry("disabled", None);
        let mut config = HashMap::new();
        config.insert(
            "disabled".to_string(),
            super::super::loader::SkillConfig {
                enabled: Some(false),
                ..Default::default()
            },
        );
        let opts = SkillLoadOptions {
            skill_config: config,
            ..Default::default()
        };
        assert!(!should_include_skill(&entry, &opts));
    }

    #[test]
    fn test_always_bypasses_requirements() {
        let meta = SkillMetadata {
            always: Some(true),
            requires: Some(SkillRequirements {
                bins: vec!["nonexistent_binary_xyz".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };
        let entry = make_entry("always-on", Some(meta));
        let opts = SkillLoadOptions::default();
        assert!(should_include_skill(&entry, &opts));
    }

    #[test]
    fn test_missing_binary_excluded() {
        let meta = SkillMetadata {
            requires: Some(SkillRequirements {
                bins: vec!["nonexistent_binary_xyz".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };
        let entry = make_entry("needs-bin", Some(meta));
        let opts = SkillLoadOptions::default();
        assert!(!should_include_skill(&entry, &opts));
    }

    #[test]
    fn test_has_binary_finds_common_tools() {
        // `ls` should exist on any Unix system
        #[cfg(unix)]
        assert!(has_binary("ls"));
    }

    #[test]
    fn test_wrong_os_excluded() {
        let meta = SkillMetadata {
            os: vec!["windows".to_string()],
            ..Default::default()
        };
        let entry = make_entry("windows-only", Some(meta));
        let opts = SkillLoadOptions::default();
        // On macOS/Linux, this should be excluded
        #[cfg(not(target_os = "windows"))]
        assert!(!should_include_skill(&entry, &opts));
    }
}
