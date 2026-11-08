//! Profile pruning integration coverage for retained skills and generated indexes.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test modules assert on known-good fixtures, so panicking macros are the intended failure signal"
)]
use super::*;
use crate::template_init::apply;
use crate::template_profile::TemplateProfile;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn profile(id: &str) -> TemplateProfile {
    TemplateProfile::load_from_path(
        &repo_root()
            .join(format!("config/template-profiles/{id}.toml"))
            .to_string_lossy(),
    )
    .unwrap()
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn profile_fixture() -> tempfile::TempDir {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    write(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\", \"benchmarks\"]\ndefault-members = [\"crates/sample-app\"]\nresolver = \"3\"\n",
    );
    write(
        &root.join("crates/example-crate/Cargo.toml"),
        "[package]\nname = \"example-crate\"\n",
    );
    write(
        &root.join("crates/example-crate/src/lib.rs"),
        "pub fn f() {}\n",
    );
    write(
        &root.join("crates/sample-app/Cargo.toml"),
        "[package]\nname = \"sample-app\"\n",
    );
    write(
        &root.join("crates/xtask/Cargo.toml"),
        "[package]\nname = \"xtask\"\n",
    );
    write(
        &root.join("config/xtask.json"),
        "{\n  \"default_tier\": \"pull-request\"\n}\n",
    );
    write(&root.join(".gitignore"), "/target/\nCargo.lock\n");
    write(
        &root.join("AGENTS.md"),
        concat!(
            "# rust-2026-template\n",
            "Reference `.agents/skills/tokio-performance/SKILL.md`.\n",
            "See `.agents/skills/stacked-prs/SKILL.md`.\n",
            "\n## Agentic Metrics Reporting\n",
            "See `.agents/skills/metrics-reporter/` for the event workflow.\n",
            "\n## Other Guidance\n",
            "Keep this section.\n",
        ),
    );
    write(&root.join(".agents/SKILLS.md"), "stale skill index\n");
    write(
        &root.join("scripts/generate-skills-md.sh"),
        include_str!("../../../../scripts/generate-skills-md.sh"),
    );

    let skill_root = repo_root().join(".agents/skills");
    for entry in fs::read_dir(skill_root).unwrap().flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        write(
            &root.join(format!(".agents/skills/{name}/SKILL.md")),
            &format!("---\nname: {name}\ndescription: Fixture skill.\n---\n"),
        );
    }

    #[cfg(unix)]
    {
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::create_dir_all(root.join(".qwen")).unwrap();
        std::os::unix::fs::symlink("../.agents/skills", root.join(".claude/skills")).unwrap();
        std::os::unix::fs::symlink("../.agents/skills", root.join(".qwen/skills")).unwrap();
    }

    fixture
}

#[test]
fn minimal_and_library_profiles_prune_skills_and_refresh_adapters() {
    for id in ["minimal", "library"] {
        let fixture = profile_fixture();
        let root = fixture.path();
        let blueprint = profile(id);
        let identity = ProjectIdentity::new(
            Some("my-app"),
            Some("My project"),
            Some("Jane Dev"),
            Some("octo/my-repo"),
        )
        .unwrap();
        let plan = InitPlan::build(root, &blueprint, &identity).unwrap();
        apply::execute(&plan).unwrap();

        let skill_root = root.join(".agents/skills");
        let retained: HashSet<_> = fs::read_dir(&skill_root)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            retained,
            CORE_SKILLS.iter().map(|name| (*name).to_string()).collect(),
            "{id} retained an unexpected skill set"
        );

        let index = fs::read_to_string(root.join(".agents/SKILLS.md")).unwrap();
        for name in CORE_SKILLS {
            assert!(
                index.contains(&format!("| `{name}` |")),
                "{id}: missing {name}"
            );
        }
        for name in [
            "dora-report",
            "metrics-reporter",
            "issue-triage",
            "skill-creator",
        ] {
            assert!(
                !retained.contains(name),
                "{id} retained maintainer skill {name}"
            );
            assert!(
                !index.contains(&format!("| `{name}` |")),
                "{id} indexed {name}"
            );
        }

        let agents = fs::read_to_string(root.join("AGENTS.md")).unwrap();
        for removed in ["tokio-performance", "stacked-prs", "metrics-reporter"] {
            assert!(
                !agents.contains(&format!(".agents/skills/{removed}/")),
                "{id} AGENTS.md references removed skill {removed}"
            );
        }
        assert!(agents.contains("## Other Guidance"));

        #[cfg(unix)]
        for adapter in [".claude/skills", ".qwen/skills"] {
            for name in CORE_SKILLS {
                assert!(root.join(adapter).join(name).is_dir());
            }
            assert!(!root.join(adapter).join("metrics-reporter").exists());
        }
    }
}
