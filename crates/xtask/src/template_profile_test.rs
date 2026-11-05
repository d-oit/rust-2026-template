//! Validation-focused tests for `template_profile` (kept out-of-line to respect the 500-LOC limit).
#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::path_rules::{is_crate_dir_name, is_safe_relative};
use std::collections::HashSet;

/// Builds a profile TOML with the given workspace section body.
fn profile_toml_with_workspace(workspace_body: &str) -> String {
    format!(
        r#"[metadata]
id = "t"
display_name = "T"
description = "d"
[workspace]
{workspace_body}
[ci]
default_tier = "pull-request"
[post_init]
checklist = ["x"]
"#
    )
}

#[test]
fn test_load_rejects_profile_id_with_traversal() {
    for bad in ["../evil", "a/b", "a\\b", "/abs", "..", ".", "con"] {
        let err = TemplateProfile::load(bad).unwrap_err();
        assert!(
            err.to_string().contains("invalid profile id"),
            "profile id '{bad}' must be rejected by id validation, got: {err}"
        );
    }
}

#[test]
fn test_validate_rejects_include_crates_traversal() {
    for bad in [
        "crates/../Cargo.toml",
        "crates/a/b",
        "crates/",
        "crates/../..",
        "/crates/x",
    ] {
        let toml = profile_toml_with_workspace(&format!("include_crates = [\"{bad}\"]"));
        let err = TemplateProfile::from_toml(&toml).unwrap_err();
        assert!(
            err.to_string().contains("include_crates"),
            "include entry '{bad}' must be rejected, got: {err}"
        );
    }
}

#[test]
fn test_validate_rejects_exclude_paths_traversal() {
    for bad in ["..", "/abs", "a/../b", "", "a\\b", ".", "sub/../.."] {
        let toml = profile_toml_with_workspace(&format!(
            "include_crates = [\"crates/xtask\"]\nexclude_paths = [\"{bad}\"]"
        ));
        let err = TemplateProfile::from_toml(&toml).unwrap_err();
        assert!(
            err.to_string().contains("exclude_paths"),
            "exclude path '{bad}' must be rejected, got: {err}"
        );
    }
}

#[test]
fn test_validate_rejects_exclude_workflows_bad() {
    for bad in ["../x.yml", "sub/x.yml", "x.sh", "..", ""] {
        let toml = profile_toml_with_workspace(&format!(
            "include_crates = [\"crates/xtask\"]\nexclude_workflows = [\"{bad}\"]"
        ));
        let err = TemplateProfile::from_toml(&toml).unwrap_err();
        assert!(
            err.to_string().contains("exclude_workflows"),
            "exclude workflow '{bad}' must be rejected, got: {err}"
        );
    }
}

#[test]
fn test_validate_rejects_metadata_id_leading_digit() {
    let toml = profile_toml_with_workspace("include_crates = [\"crates/xtask\"]")
        .replace("id = \"t\"", "id = \"1bad\"");
    let err = TemplateProfile::from_toml(&toml).unwrap_err();
    assert!(err.to_string().contains("metadata.id"));
}

#[test]
fn test_is_safe_relative_rejects_component_attacks() {
    assert!(is_safe_relative("benchmarks"));
    assert!(is_safe_relative("docs/patterns"));
    assert!(!is_safe_relative(""));
    assert!(!is_safe_relative(".."));
    assert!(!is_safe_relative("."));
    assert!(!is_safe_relative("/abs"));
    assert!(!is_safe_relative("a/../b"));
    assert!(!is_safe_relative("a\\b"));
    assert!(!is_safe_relative("a\u{0}b"));

    // Bidi controls, line/paragraph separators, and zero-width spaces must be rejected
    assert!(!is_safe_relative("a\u{200b}b"));
    assert!(!is_safe_relative("a\u{2028}b"));
    assert!(!is_safe_relative("a\u{2029}b"));
    assert!(!is_safe_relative("a\u{202a}b"));
    assert!(!is_safe_relative("a\u{202e}b"));
    assert!(!is_safe_relative("a\u{2066}b"));
    assert!(!is_safe_relative("a\u{2069}b"));
}

#[test]
fn test_shipped_profiles_lockfile_policies() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    for id in SHIPPED_PROFILES {
        let path = repo_root
            .join(format!("{PROFILES_DIR}/{id}.toml"))
            .to_string_lossy()
            .into_owned();
        let profile = TemplateProfile::load_from_path(&path)
            .unwrap_or_else(|e| panic!("profile {id} must load: {e}"));
        if *id == "library" {
            assert_eq!(
                profile.policy.lockfile,
                LockfilePolicy::Ignored,
                "library profile must have LockfilePolicy::Ignored"
            );
            assert!(
                !profile
                    .post_init
                    .checklist
                    .contains(&"commit-cargo-lock".to_string()),
                "library profile checklist must not instruct to commit cargo lock"
            );
        } else {
            assert_eq!(
                profile.policy.lockfile,
                LockfilePolicy::Committed,
                "profile '{id}' must have LockfilePolicy::Committed"
            );
            assert!(
                profile
                    .post_init
                    .checklist
                    .contains(&"commit-cargo-lock".to_string()),
                "profile '{id}' checklist must contain commit-cargo-lock"
            );
        }
    }
}

#[test]
fn test_cli_is_a_compatibility_alias_with_its_original_policy() {
    let cli = TemplateProfile::load("cli").unwrap();
    let minimal_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(format!("{PROFILES_DIR}/minimal.toml"));
    let minimal = TemplateProfile::load_from_path(&minimal_path.to_string_lossy()).unwrap();

    assert!(!SHIPPED_PROFILES.contains(&"cli"));
    assert_eq!(cli.metadata.id, "cli");
    assert_eq!(
        cli.workspace.include_crates,
        minimal.workspace.include_crates
    );
    assert_eq!(cli.workspace.exclude_paths, minimal.workspace.exclude_paths);
    assert_eq!(
        cli.workspace.exclude_workflows,
        minimal.workspace.exclude_workflows
    );
    assert_eq!(cli.ci.default_tier, "protected-branch");
    assert_eq!(cli.policy.lockfile, LockfilePolicy::Committed);
    assert!(
        cli.post_init
            .checklist
            .contains(&"generate-cli-manpage".to_string())
    );
}

#[test]
fn test_minimal_and_library_exclude_non_core_skill_directories() {
    const CORE_SKILLS: &[&str] = &[
        "anti-ai-slop",
        "build-rust",
        "crates-io-name-check",
        "harness",
        "lint-rust",
        "privacy-first",
        "release-rust",
        "reviewing-pull-requests",
        "secret-lint",
        "test-rust",
        "verify-actions",
    ];

    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let core_paths: HashSet<_> = CORE_SKILLS
        .iter()
        .map(|name| format!(".agents/skills/{name}"))
        .collect();
    let skill_root = repo_root.join(".agents/skills");
    let skill_names: HashSet<_> = std::fs::read_dir(skill_root)
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();

    for id in ["minimal", "library"] {
        let profile = TemplateProfile::load_from_path(
            &repo_root
                .join(format!("{PROFILES_DIR}/{id}.toml"))
                .to_string_lossy(),
        )
        .unwrap();
        let excluded: HashSet<_> = profile.workspace.exclude_paths.iter().cloned().collect();
        let retained: HashSet<_> = skill_names
            .iter()
            .filter(|name| !excluded.contains(&format!(".agents/skills/{name}")))
            .map(|name| format!(".agents/skills/{name}"))
            .collect();

        assert_eq!(
            retained, core_paths,
            "{id} should keep exactly the declared core Rust skills"
        );
    }

    let load_shipped = |id: &str| {
        TemplateProfile::load_from_path(
            &repo_root
                .join(format!("{PROFILES_DIR}/{id}.toml"))
                .to_string_lossy(),
        )
        .unwrap()
    };
    let retained_for = |id: &str| {
        let excluded: HashSet<_> = load_shipped(id)
            .workspace
            .exclude_paths
            .into_iter()
            .collect();
        skill_names
            .iter()
            .filter(|name| !excluded.contains(&format!(".agents/skills/{name}")))
            .cloned()
            .collect::<HashSet<_>>()
    };
    let maintainer_skills = [
        "architecture-diagram",
        "codacy",
        "dora-report",
        "issue-triage",
        "metrics-reporter",
        "stacked-prs",
    ];
    for id in ["service", "performance-sensitive"] {
        let retained = retained_for(id);
        for name in maintainer_skills {
            assert!(!retained.contains(name), "{id} retained {name}");
        }
        for name in ["goap-agent", "tokio-performance", "triz-analysis"] {
            assert!(
                retained.contains(name),
                "{id} omitted advanced skill {name}"
            );
        }
    }

    let agent_skills = retained_for("ai-agent");
    for name in ["dora-report", "issue-triage", "metrics-reporter"] {
        assert!(!agent_skills.contains(name), "ai-agent retained {name}");
    }
    for name in ["skill-creator", "skill-evaluator"] {
        assert!(agent_skills.contains(name), "ai-agent omitted {name}");
    }
    assert_eq!(retained_for("workspace"), skill_names);
}

#[test]
fn test_shipped_profiles_keep_local_path_dependencies() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let root_manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(root.join("Cargo.toml")).unwrap()).unwrap();
    let workspace_dependencies = root_manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
        .unwrap();
    let crates_root = root.join("crates").canonicalize().unwrap();

    for id in SHIPPED_PROFILES {
        let profile_file = root
            .join(format!("{PROFILES_DIR}/{id}.toml"))
            .to_string_lossy()
            .into_owned();
        let profile = TemplateProfile::load_from_path(&profile_file)
            .unwrap_or_else(|e| panic!("profile {id} must load: {e}"));
        let included: HashSet<_> = profile.workspace.include_crates.iter().cloned().collect();

        for crate_path in &profile.workspace.include_crates {
            let crate_dir = root.join(crate_path);
            let crate_manifest = crate_dir.join("Cargo.toml");
            // An initialized adopter has already removed crates from other
            // profiles; validate the retained manifests that are still present.
            if !crate_manifest.is_file() {
                continue;
            }
            let manifest: toml::Value =
                toml::from_str(&std::fs::read_to_string(crate_manifest).unwrap()).unwrap();

            for dependencies in dependency_tables(&manifest) {
                for (name, specification) in dependencies {
                    let inherited = specification
                        .get("workspace")
                        .and_then(toml::Value::as_bool)
                        == Some(true);
                    let workspace_specification = workspace_dependencies.get(name);
                    let path = specification
                        .get("path")
                        .and_then(toml::Value::as_str)
                        .or_else(|| {
                            inherited
                                .then_some(workspace_specification)
                                .flatten()
                                .and_then(|spec| spec.get("path"))
                                .and_then(toml::Value::as_str)
                        });
                    let Some(path) = path else {
                        continue;
                    };
                    let base = if inherited { &root } else { &crate_dir };
                    let dependency = base.join(path).canonicalize().unwrap_or_else(|error| {
                        panic!(
                            "profile {id}: local dependency {name} path {path} is invalid: {error}"
                        )
                    });
                    let Ok(relative) = dependency.strip_prefix(&crates_root) else {
                        continue;
                    };
                    let dependency_path =
                        format!("crates/{}", relative.to_string_lossy().replace('\\', "/"));
                    assert!(
                        included.contains(&dependency_path),
                        "profile '{id}' keeps '{crate_path}' but prunes local path dependency '{dependency_path}'"
                    );
                }
            }
        }
    }
}

fn dependency_tables(manifest: &toml::Value) -> Vec<&toml::map::Map<String, toml::Value>> {
    const GROUPS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

    fn push_group<'a>(
        manifest: &'a toml::Value,
        group: &str,
        tables: &mut Vec<&'a toml::map::Map<String, toml::Value>>,
    ) {
        if let Some(table) = manifest.get(group).and_then(toml::Value::as_table) {
            tables.push(table);
        }
    }

    let mut tables = Vec::new();
    for group in GROUPS {
        push_group(manifest, group, &mut tables);
    }
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            for group in GROUPS {
                push_group(target, group, &mut tables);
            }
        }
    }
    tables
}

#[test]
fn test_is_crate_dir_name_rules() {
    for ok in ["example-crate", "a", "sample-app2"] {
        assert!(is_crate_dir_name(ok), "{ok} must be valid");
    }
    for bad in [
        "",
        "-x",
        "X",
        "has_underscore",
        "has/slash",
        "a..b",
        "toolongx".repeat(20).as_str(),
    ] {
        assert!(!is_crate_dir_name(bad), "{bad} must be invalid");
    }
}
