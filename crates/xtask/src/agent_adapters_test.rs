#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use std::fs;
use std::path::Path;

const RMARK: &str = "@AGENTS.md";

fn base_toml(adapter: &str) -> String {
    format!(
        r#"[contract]
canonical_instructions = "AGENTS.md"
skills_directory = ".agents/skills"
context_files = ["llms.txt"]
[validation]
require_canonical_reference = true
reject_policy_duplication = true
verify_local_links = true
enforce_adapter_scope = true
max_agents_md_lines = 200
{adapter}"#
    )
}
fn adapter(id: &str, ep: &str) -> String {
    format!(
        r#"[[adapters]]
id = "{id}"
root = ".{id}"
entrypoint = "{ep}"
role = "tool-delta"
canonical_reference = "AGENTS.md""#
    )
}

#[test]
fn test_parse_valid_manifest() {
    let m = AgentAdaptersManifest::from_toml(&base_toml(&adapter("claude", "CLAUDE.md"))).unwrap();
    assert_eq!(m.contract.canonical_instructions, "AGENTS.md");
    assert_eq!(m.adapters[0].id, "claude");
}

#[test]
fn test_parse_rejects_unknown_fields() {
    let t = base_toml(&adapter("x", "X.md")).replace(
        "context_files = [\"llms.txt\"]",
        "context_files = []\nstray = true",
    );
    assert!(AgentAdaptersManifest::from_toml(&t).is_err());
}

#[test]
fn test_requires_adapters() {
    let t = base_toml("").replace("context_files = [\"llms.txt\"]", "context_files = []");
    assert!(AgentAdaptersManifest::from_toml(&t).is_err());
}

#[test]
fn test_inventory_markdown() {
    let m = AgentAdaptersManifest::from_toml(&base_toml(&adapter("claude", "CLAUDE.md"))).unwrap();
    let md = m.inventory_markdown();
    assert!(md.contains("claude") && md.contains("tool-delta"));
}

#[test]
fn test_finds_missing_entrypoint() {
    let m = AgentAdaptersManifest::from_toml(&base_toml(&adapter("ghost", "NOPE.md"))).unwrap();
    let r = m.validate(Path::new(".")).unwrap();
    assert!(!r.is_ok());
    assert!(r.errors.iter().any(|e| e.message.contains("NOPE.md")));
}

#[test]
fn test_finds_invalid_id() {
    let t = base_toml(&adapter("Bad", "B.md")).replace(
        "require_canonical_reference = true",
        "require_canonical_reference = false",
    );
    let m = AgentAdaptersManifest::from_toml(&t).unwrap();
    let r = m.validate(Path::new(".")).unwrap();
    assert!(!r.is_ok());
    assert!(r.errors.iter().any(|e| e.source == "Bad"));
}

#[test]
fn test_print_report() {
    let r = ValidationResult {
        errors: vec![err("t", "e")],
        warnings: vec![warn("t", "w")],
    };
    assert!(!r.is_ok());
    r.print_report();
}

#[test]
fn test_uses_canonical_reference_field() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("T.md"), format!("# T\n{RMARK}\n")).unwrap();
    let t = base_toml(
        r#"[[adapters]]
id = "test"
root = ""
entrypoint = "T.md"
role = "tool-delta"
canonical_reference = "FOO.md""#,
    )
    .replace(
        "reject_policy_duplication = true",
        "reject_policy_duplication = false",
    )
    .replace(
        "enforce_adapter_scope = true",
        "enforce_adapter_scope = false",
    );
    let m = AgentAdaptersManifest::from_toml(&t).unwrap();
    let r = m.validate(dir.path()).unwrap();
    assert!(!r.is_ok());
    assert!(r.errors.iter().any(|e| e.message.contains("@FOO.md")));
}

#[test]
fn test_read_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("big.md");
    fs::write(&p, "x".repeat(2_097_152)).unwrap();
    assert!(read_bounded(&p, 1024).unwrap().len() <= 1024);
}
