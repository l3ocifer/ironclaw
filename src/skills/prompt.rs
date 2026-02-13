//! Skills prompt formatting.
//!
//! Formats eligible skills into the XML format used in the system prompt.
//! Follows the Agent Skills standard: https://agentskills.io/integrate-skills

use super::Skill;

/// Format skills for inclusion in the system prompt.
///
/// Uses XML format per the Agent Skills standard. Only name, description,
/// and file location are included — the full SKILL.md content is loaded
/// on-demand by the agent when a task matches (progressive disclosure).
///
/// Skills with `disable_model_invocation = true` are excluded.
pub fn format_skills_for_prompt(skills: &[&Skill]) -> String {
    let visible: Vec<&&Skill> = skills
        .iter()
        .filter(|s| !s.disable_model_invocation)
        .collect();

    if visible.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        String::new(),
        String::new(),
        "The following skills provide specialized instructions for specific tasks.".to_string(),
        "Use the read tool to load a skill's file when the task matches its description."
            .to_string(),
        String::new(),
        "<available_skills>".to_string(),
    ];

    for skill in visible {
        lines.push("  <skill>".to_string());
        lines.push(format!("    <name>{}</name>", escape_xml(&skill.name)));
        lines.push(format!(
            "    <description>{}</description>",
            escape_xml(&skill.description)
        ));
        lines.push(format!(
            "    <location>{}</location>",
            escape_xml(&skill.file_path.to_string_lossy())
        ));
        lines.push("  </skill>".to_string());
    }

    lines.push("</available_skills>".to_string());
    lines.join("\n")
}

/// Escape XML special characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_skill(name: &str, description: &str) -> Skill {
        Skill {
            name: name.to_string(),
            description: description.to_string(),
            file_path: PathBuf::from(format!("/home/user/.ironclaw/skills/{}/SKILL.md", name)),
            base_dir: PathBuf::from(format!("/home/user/.ironclaw/skills/{}", name)),
            source: "test".to_string(),
            disable_model_invocation: false,
        }
    }

    #[test]
    fn test_format_empty() {
        let skills: Vec<&Skill> = vec![];
        assert_eq!(format_skills_for_prompt(&skills), "");
    }

    #[test]
    fn test_format_single_skill() {
        let skill = make_skill("weather", "Get weather forecasts.");
        let prompt = format_skills_for_prompt(&[&skill]);
        assert!(prompt.contains("<available_skills>"));
        assert!(prompt.contains("<name>weather</name>"));
        assert!(prompt.contains("<description>Get weather forecasts.</description>"));
        assert!(prompt.contains("</available_skills>"));
    }

    #[test]
    fn test_format_multiple_skills() {
        let s1 = make_skill("weather", "Get weather forecasts.");
        let s2 = make_skill("github", "GitHub CLI integration.");
        let prompt = format_skills_for_prompt(&[&s1, &s2]);
        assert!(prompt.contains("<name>weather</name>"));
        assert!(prompt.contains("<name>github</name>"));
    }

    #[test]
    fn test_format_excludes_disabled() {
        let mut skill = make_skill("secret", "Hidden skill.");
        skill.disable_model_invocation = true;
        let prompt = format_skills_for_prompt(&[&skill]);
        assert!(prompt.is_empty());
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("foo & bar"), "foo &amp; bar");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
    }
}
