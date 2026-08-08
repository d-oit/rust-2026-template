//! Doctor / Toolchain checks.

use crate::config::XtaskError;
use std::path::Path;
use std::process::Command;

/// Struct representing a tool to check.
pub struct Tool {
    /// Name of the tool.
    pub name: &'static str,
    /// The executable name to run.
    pub command: &'static str,
    /// Arguments to test the tool.
    pub args: &'static [&'static str],
    /// Whether the tool is strictly required.
    pub required: bool,
    /// GUIDANCE instructions to install the tool.
    pub guidance: &'static str,
}

const TOOLS: &[Tool] = &[
    Tool {
        name: "git",
        command: "git",
        args: &["--version"],
        required: true,
        guidance: "Install Git from https://git-scm.com/",
    },
    Tool {
        name: "cargo",
        command: "cargo",
        args: &["--version"],
        required: true,
        guidance: "Install Rust and Cargo from https://rustup.rs/",
    },
    Tool {
        name: "cargo-nextest",
        command: "cargo",
        args: &["nextest", "--version"],
        required: false,
        guidance: "Install via: cargo install cargo-nextest",
    },
    Tool {
        name: "cargo-audit",
        command: "cargo",
        args: &["audit", "--version"],
        required: false,
        guidance: "Install via: cargo install cargo-audit",
    },
    Tool {
        name: "cargo-deny",
        command: "cargo",
        args: &["deny", "--version"],
        required: false,
        guidance: "Install via: cargo install cargo-deny",
    },
    Tool {
        name: "cargo-machete",
        command: "cargo",
        args: &["machete", "--version"],
        required: false,
        guidance: "Install via: cargo install cargo-machete",
    },
    Tool {
        name: "shellcheck",
        command: "shellcheck",
        args: &["--version"],
        required: false,
        guidance: "Install via package manager (e.g. 'sudo apt install shellcheck' or 'brew install shellcheck')",
    },
    Tool {
        name: "markdownlint-cli2",
        command: "markdownlint-cli2",
        args: &["--version"],
        required: false,
        guidance: "Install via npm: npm install -g markdownlint-cli2",
    },
];

/// Run all doctor/environment checks and print diagnostic results.
///
/// # Errors
/// Returns `XtaskError::MissingTool` if any required tool is missing.
pub fn run_doctor() -> Result<(), XtaskError> {
    println!("=== Environment Diagnostics ===");
    println!();

    let mut missing_required = Vec::new();
    let mut missing_optional = Vec::new();

    println!("Required & Optional Tools:");
    for tool in TOOLS {
        let available = Command::new(tool.command)
            .args(tool.args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .and_then(|mut child| child.wait())
            .map(|status| status.success())
            .unwrap_or(false);

        if available {
            println!("  ✓ {}: installed", tool.name);
        } else if tool.required {
            println!("  ✗ {} (REQUIRED): NOT FOUND", tool.name);
            missing_required.push(tool);
        } else {
            println!("  ! {} (OPTIONAL): NOT FOUND", tool.name);
            missing_optional.push(tool);
        }
    }
    println!();

    // Linker configuration check
    println!("Linker Configuration:");
    check_linker();
    println!();

    // Git state check
    println!("Git State:");
    check_git_state();
    println!();

    // Symlinks
    println!("Skill symlinks:");
    check_symlinks();
    println!();

    // Git hooks
    println!("Git Hooks:");
    check_git_hooks();
    println!();

    // Core files check
    println!("Core Files:");
    check_core_files();
    println!();

    // Skills
    println!("Skills count:");
    check_skills_count();
    println!();

    if !missing_required.is_empty() {
        let first = missing_required[0];
        return Err(XtaskError::MissingTool {
            tool_name: first.name.to_string(),
            guidance: first.guidance.to_string(),
        });
    }

    if !missing_optional.is_empty() {
        println!("Guidance for missing optional tools:");
        for tool in missing_optional {
            println!("  - {}: {}", tool.name, tool.guidance);
        }
        println!();
    }

    println!("All required checks completed successfully.");
    Ok(())
}

fn check_linker() {
    let os = std::env::consts::OS;
    match os {
        "linux" => {
            let has_mold = Command::new("mold")
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok();
            let has_clang = Command::new("clang")
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok();

            if has_mold && has_clang {
                println!("  ✓ mold + clang detected — maximum link speed");
            } else {
                println!("  ! mold or clang missing — compile times might be slower");
                println!("    Guidance: Install mold + clang (e.g., 'sudo apt install mold clang')");
            }
        }
        "macos" => {
            println!("  ✓ Using default macOS linker (ld64 — already optimized)");
        }
        "windows" => {
            let has_lld = Command::new("rust-lld")
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok();
            if has_lld {
                println!("  ✓ rust-lld detected");
            } else {
                println!("  ! rust-lld not found — should ship with Rust toolchain");
            }
        }
        other => {
            println!("  ! Unknown platform linker check for OS: {other}");
        }
    }
}

fn check_git_state() {
    let branch_output = Command::new("git")
        .args(["branch", "--show-current"])
        .output();
    match branch_output {
        Ok(out) if out.status.success() => {
            let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let branch_name = if branch.is_empty() { "detached HEAD".to_string() } else { branch };
            println!("  ✓ Current branch: {branch_name}");
        }
        _ => {
            println!("  ! Could not query git branch name");
        }
    }

    let status_output = Command::new("git")
        .args(["diff", "--quiet"])
        .status();
    let status_cached_output = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status();

    match (status_output, status_cached_output) {
        (Ok(s1), Ok(s2)) if s1.success() && s2.success() => {
            println!("  ✓ Working tree clean");
        }
        _ => {
            println!("  ! Working tree has uncommitted changes");
        }
    }
}

fn check_symlinks() {
    for cli_dir in &[".claude/skills", ".qwen/skills"] {
        let path = Path::new(cli_dir);
        if path.exists() {
            let mut count = 0;
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Ok(meta) = entry.file_type() {
                        if meta.is_symlink() {
                            count += 1;
                        }
                    }
                }
            }
            if count > 0 {
                println!("  ✓ {cli_dir}: {count} symlinks");
            } else {
                println!("  ! {cli_dir}: no symlinks (run ./scripts/setup-skills.sh)");
            }
        } else {
            println!("  ! {cli_dir}: directory missing");
        }
    }
}

fn check_git_hooks() {
    let hooks_output = Command::new("git")
        .args(["config", "core.hooksPath"])
        .output();
    match hooks_output {
        Ok(out) if out.status.success() => {
            let hooks_path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if hooks_path == ".githooks" {
                if Path::new(".githooks/pre-commit").exists() {
                    println!("  ✓ pre-commit hook installed");
                } else {
                    println!("  ! core.hooksPath set but .githooks/pre-commit missing");
                }
            } else {
                println!("  ! core.hooksPath is set to '{hooks_path}', expected '.githooks'");
            }
        }
        _ => {
            println!("  ! core.hooksPath not set to .githooks (run: git config core.hooksPath .githooks)");
        }
    }
}

fn check_core_files() {
    let core_files = &["AGENTS.md", "CHANGELOG.md", "Cargo.toml", "rust-toolchain.toml", "deny.toml"];
    for file in core_files {
        if Path::new(file).exists() {
            println!("  ✓ {file}");
        } else {
            println!("  ✗ {file}: missing");
        }
    }
}

fn check_skills_count() {
    let path = Path::new(".agents/skills");
    if path.exists() {
        let mut skill_count = 0;
        // Search skills directory for SKILL.md
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let skill_path = entry.path().join("SKILL.md");
                if skill_path.exists() {
                    skill_count += 1;
                }
            }
        }
        println!("  ✓ .agents/skills: {skill_count} skills");
    } else {
        println!("  ✗ .agents/skills: directory missing");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_linker() {
        check_linker();
    }

    #[test]
    fn test_tools_list() {
        assert!(TOOLS.len() >= 2);
        assert_eq!(TOOLS[0].name, "git");
        assert_eq!(TOOLS[1].name, "cargo");
    }
}
