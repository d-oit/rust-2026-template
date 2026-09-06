//! Plan-building and end-to-end tests for template initialization.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::super::apply;
use super::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const PROFILE_TOML: &str = r#"
[metadata]
id = "t"
display_name = "T"
description = "d"
[workspace]
include_crates = ["crates/example-crate"]
exclude_paths = ["benchmarks"]
exclude_workflows = ["removed.yml"]
[ci]
default_tier = "protected-branch"
[policy]
lockfile = "committed"
publish_packages = []
[post_init]
checklist = ["item-a"]
"#;

fn identity() -> ProjectIdentity {
    ProjectIdentity::new(
        Some("my-app"),
        Some("My project"),
        Some("Jane Dev"),
        Some("octo/my-repo"),
    )
    .unwrap()
}

/// Fixture tree written by `write_fixture`. Held as static data (not
/// expression-position literals) so no string is const-promoted onto the stack
/// through generic `AsRef` boundaries.
static FIXTURE_FILES: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\", \"benchmarks\"]\ndefault-members = [\"crates/sample-app\"]\nresolver = \"3\"\n\n[workspace.package]\nversion = \"0.0.0\"\n# keep this comment\nauthors = [\"Your Name\"]\nrepository = \"https://github.com/your-org/your-repo\"\nhomepage = \"https://github.com/your-org/your-repo\"\n",
    ),
    (
        "crates/example-crate/Cargo.toml",
        "[package]\nname = \"example-crate\"\ndescription = \"Example crate\"\n",
    ),
    (
        "crates/example-crate/src/lib.rs",
        "pub const CRATE_ID: &str = \"example_crate\";\n",
    ),
    ("crates/example-crate/README.md", "# example-crate\n"),
    (
        "crates/sample-app/Cargo.toml",
        "[package]\nname = \"sample-app\"\n",
    ),
    ("crates/sample-app/src/main.rs", "fn main() {}\n"),
    ("benchmarks/placeholder.txt", "x\n"),
    (".github/workflows/removed.yml", "on: []\n"),
    (
        "config/xtask.json",
        "{\n  \"default_tier\": \"pull-request\"\n}\n",
    ),
    (
        ".gitignore",
        "/target/\n\n# Cargo.lock is intentionally excluded for this library/template repo.\n# If you adopt this template for a *binary* application, remove this line\n# and commit your Cargo.lock. See README.md#cargo-lock-policy for details.\nCargo.lock\n",
    ),
    ("AGENTS.md", "# rust-2026-template\n"),
];

fn write_fixture(root: &Path) {
    for (rel, content) in FIXTURE_FILES {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
    }
}

fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    write_fixture(dir.path());
    let root = dir.path().to_path_buf();
    (dir, root)
}

fn build_plan(root: &Path) -> InitPlan {
    let blueprint = TemplateProfile::from_toml(PROFILE_TOML).unwrap();
    InitPlan::build(root, &blueprint, &identity()).unwrap()
}

fn snapshot(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for entry in walk(root) {
        let rel = entry
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        if entry.is_dir() {
            out.insert(format!("{rel}/"), String::new());
        } else {
            out.insert(
                rel,
                fs::read_to_string(&entry).unwrap_or_else(|_| "<binary>".into()),
            );
        }
    }
    out
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).unwrap().flatten() {
        paths.push(entry.path());
        if entry.path().is_dir() {
            paths.extend(walk(&entry.path()));
        }
    }
    paths
}

fn rel_paths(plan: &InitPlan) -> Vec<String> {
    plan.removals
        .iter()
        .map(|p| {
            p.strip_prefix(&plan.root)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

// Ported from upstream `prune_default_members` tests (issue #321) to prove the
// structural `toml_edit` implementation preserves the same contract.
mod default_members_contract {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;

    fn identity() -> ProjectIdentity {
        ProjectIdentity::new(
            Some("my-app"),
            Some("My project"),
            Some("Jane Dev"),
            Some("octo/my-repo"),
        )
        .unwrap()
    }

    fn removed(names: &[&str]) -> Vec<String> {
        names.iter().copied().map(str::to_string).collect()
    }

    #[test]
    fn default_members_pruned_when_profile_removes_crate() {
        let manifest = concat!(
            "[workspace]\n",
            "members = [\"crates/*\", \"examples/*\", \"benchmarks\"]\n",
            "default-members = [\"crates/sample-app\"]\n",
            "resolver = \"3\"\n"
        );
        let removed = removed(&["crates/sample-app"]);
        let updated = edit_workspace_manifest(manifest, &removed, &identity())
            .unwrap()
            .unwrap();
        assert!(
            !updated.contains("default-members"),
            "dangling key must be dropped: {updated}"
        );
        assert!(
            updated.contains("members = [\"crates/*\", \"examples/*\", \"benchmarks\"]"),
            "members must survive: {updated}"
        );
        assert!(
            updated.contains("resolver = \"3\""),
            "unrelated lines must survive: {updated}"
        );
    }

    #[test]
    fn default_members_survive_when_crate_not_removed() {
        let manifest = "[workspace]\ndefault-members = [\"crates/sample-app\"]\n";
        let removed = removed(&["crates/other-crate"]);
        assert!(
            edit_workspace_manifest(manifest, &removed, &identity())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn default_members_partially_pruned_when_others_remain() {
        let manifest = "[workspace]\ndefault-members = [\"crates/sample-app\", \"crates/xtask\"]\n";
        let removed = removed(&["crates/sample-app"]);
        let updated = edit_workspace_manifest(manifest, &removed, &identity())
            .unwrap()
            .unwrap();
        // Structural (toml_edit) rewrite keeps valid TOML; assert the contract,
        // not byte-exact formatting.
        assert!(
            updated.contains("default-members"),
            "key must survive: {updated}"
        );
        assert!(
            updated.contains("\"crates/xtask\""),
            "kept entry must survive: {updated}"
        );
        assert!(
            !updated.contains("crates/sample-app"),
            "removed entry must go: {updated}"
        );
    }

    #[test]
    fn removed_literal_members_are_pruned_structurally() {
        let manifest = "[workspace]\nmembers = [\"crates/*\", \"benchmarks\"]\n";
        let removed = removed(&["benchmarks"]);
        let updated = edit_workspace_manifest(manifest, &removed, &identity())
            .unwrap()
            .unwrap();
        assert!(
            updated.contains("\"crates/*\""),
            "glob member must survive: {updated}"
        );
        assert!(
            !updated.contains("benchmarks"),
            "removed member must go: {updated}"
        );
    }
}

#[test]
fn build_plan_does_not_mutate_fixture() {
    let (_dir, root) = fixture();
    let before = snapshot(&root);
    let _plan = build_plan(&root);
    assert_eq!(
        before,
        snapshot(&root),
        "planning must not mutate the fixture"
    );
}

#[test]
fn plan_is_deterministic() {
    let (_dir, root) = fixture();
    let a = build_plan(&root);
    let b = build_plan(&root);
    assert_eq!(a, b);
}

#[test]
fn plan_computes_expected_operations() {
    let (_dir, root) = fixture();
    let plan = build_plan(&root);

    let removals = rel_paths(&plan);
    assert!(removals.contains(&"crates/sample-app".to_string()));
    assert!(removals.contains(&"benchmarks".to_string()));
    assert!(removals.contains(&".github/workflows/removed.yml".to_string()));

    let rename = plan.rename.as_ref().unwrap();
    assert!(rename.from.ends_with("crates/example-crate"));
    assert!(rename.to.ends_with("crates/my-app"));

    let manifest = plan.workspace_manifest.as_ref().unwrap().content.clone();
    assert!(
        !manifest.contains("benchmarks"),
        "removed member must go: {manifest}"
    );
    assert!(
        !manifest.contains("default-members"),
        "emptied default-members key must go"
    );
    assert!(manifest.contains("crates/*"), "glob members survive");
    assert!(manifest.contains("# keep this comment"), "comments survive");
    assert!(manifest.contains("authors = [\"Jane Dev\"]"));
    assert!(manifest.contains("https://github.com/octo/my-repo"));

    let crate_manifest = plan.crate_manifest.as_ref().unwrap().content.clone();
    assert!(crate_manifest.contains("name = \"my-app\""));
    assert!(crate_manifest.contains("description = \"My project\""));
    assert!(!crate_manifest.contains("example-crate"));

    let ci = plan.ci_config.as_ref().unwrap().content.clone();
    assert!(ci.contains("protected-branch"));

    let gitignore = plan.gitignore.as_ref().unwrap().content.clone();
    assert!(!gitignore.contains("Cargo.lock"));
    assert!(gitignore.contains("/target/"));

    let agents = plan
        .text_replacements
        .iter()
        .find(|r| r.path.ends_with("AGENTS.md"))
        .expect("AGENTS.md rewrite planned");
    assert_eq!(agents.content, "# my-app\n");

    let lib = plan
        .text_replacements
        .iter()
        .find(|r| r.path.ends_with("crates/my-app/src/lib.rs"))
        .expect("renamed crate lib.rs rewrite planned");
    assert!(lib.content.contains("my_app"));
}

#[test]
fn plan_rejects_existing_rename_destination() {
    let (_dir, root) = fixture();
    fs::create_dir_all(root.join("crates/taken")).unwrap();
    let blueprint = TemplateProfile::from_toml(PROFILE_TOML).unwrap();
    let id = ProjectIdentity::new(
        Some("taken"),
        Some("My project"),
        Some("Jane Dev"),
        Some("octo/my-repo"),
    )
    .unwrap();
    let err = InitPlan::build(&root, &blueprint, &id).unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[cfg(unix)]
#[test]
fn plan_rejects_symlinked_removal_escape() {
    let (_dir, root) = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::remove_dir_all(root.join("benchmarks")).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.join("benchmarks")).unwrap();
    let blueprint = TemplateProfile::from_toml(PROFILE_TOML).unwrap();
    let err = InitPlan::build(&root, &blueprint, &identity()).unwrap_err();
    assert!(err.to_string().contains("outside the repository root"));
}

#[test]
fn execute_applies_the_plan_end_to_end() {
    let (_dir, root) = fixture();
    let plan = build_plan(&root);
    apply::execute(&plan).unwrap();

    assert!(!root.join("crates/example-crate").exists());
    assert!(!root.join("crates/sample-app").exists());
    assert!(!root.join("benchmarks").exists());
    assert!(!root.join(".github/workflows/removed.yml").exists());

    let lib = fs::read_to_string(root.join("crates/my-app/src/lib.rs")).unwrap();
    assert!(lib.contains("my_app"));
    assert!(!lib.contains("example_crate"));

    let crate_manifest = fs::read_to_string(root.join("crates/my-app/Cargo.toml")).unwrap();
    assert!(crate_manifest.contains("name = \"my-app\""));

    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("Jane Dev"));
    assert!(!manifest.contains("benchmarks"));

    let agents = fs::read_to_string(root.join("AGENTS.md")).unwrap();
    assert_eq!(agents, "# my-app\n");

    let ci = fs::read_to_string(root.join("config/xtask.json")).unwrap();
    assert!(ci.contains("protected-branch"));

    let gitignore = fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(!gitignore.contains("Cargo.lock"));

    let leftovers: Vec<String> = walk(&root)
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| p.contains("xtask-tmp"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temp files must not leak: {leftovers:?}"
    );
}
