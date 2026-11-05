/// Removes links to profile-pruned skills from generated agent instructions.
///
/// The metrics reporting section is removed as a whole when its skill is
/// excluded, because the section's instructions no longer apply downstream.
pub(super) fn prune_skill_references(content: &str, removed_paths: &[String]) -> String {
    let removed_skills: Vec<_> = removed_paths
        .iter()
        .filter_map(|path| path.strip_prefix(".agents/skills/"))
        .collect();
    if removed_skills.is_empty() {
        return content.to_string();
    }

    let remove_metrics_section = removed_skills.contains(&"metrics-reporter");
    let mut in_metrics_section = false;
    let mut kept = Vec::new();
    for line in content.lines() {
        if in_metrics_section {
            if line.starts_with("## ") {
                in_metrics_section = false;
            } else {
                continue;
            }
        }

        if remove_metrics_section && line == "## Agentic Metrics Reporting" {
            in_metrics_section = true;
            continue;
        }
        if removed_skills
            .iter()
            .any(|skill| line.contains(&format!(".agents/skills/{skill}")))
        {
            continue;
        }
        kept.push(line);
    }

    let mut result = kept.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}
