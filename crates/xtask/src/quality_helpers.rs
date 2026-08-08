//! Helper functions for quality gate checks (file scanning, line counting, privacy, and secrets).
#![allow(clippy::unwrap_used)]

use crate::commands;
use crate::config::XtaskError;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Helper to recursively find files with a given extension.
pub fn find_files(dir: &Path, ext: &str, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name != "target" && name != ".git" && name != ".cargo" && name != "node_modules" {
                    find_files(&path, ext, files);
                }
            } else if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some(ext) {
                files.push(path);
            }
        }
    }
}

/// Helper to recursively find all files.
pub fn find_files_all(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name != "target" && name != ".git" && name != ".cargo" && name != "node_modules" {
                    find_files_all(&path, files);
                }
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
}

/// Helper to count the number of lines in a file.
///
/// # Errors
/// Returns `XtaskError` if opening the file fails.
pub fn count_lines(path: &Path) -> Result<usize, XtaskError> {
    let file = File::open(path).map_err(|e| XtaskError::InvalidConfig {
        message: format!("Failed to open file for line counting: {e}"),
    })?;
    let reader = BufReader::new(file);
    let mut count = 0;
    for _ in reader.lines() {
        count += 1;
    }
    Ok(count)
}

/// Runs privacy check scanning for non-test emails.
///
/// # Errors
/// Returns `XtaskError` if privacy leak is found.
pub fn run_privacy_check() -> Result<(), XtaskError> {
    let email_re = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
        .map_err(|e| XtaskError::InvalidConfig {
            message: e.to_string(),
        })?;

    let exclude_re = Regex::new(r"example\.com|example\.org|test\.com|\.git|target|\.opencode|\.mimocode|\.cargo|node_modules")
        .map_err(|e| XtaskError::InvalidConfig {
            message: e.to_string(),
        })?;

    let mut files = Vec::new();
    let is_git = Command::new("git")
        .arg("rev-parse")
        .arg("--git-dir")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if is_git {
        if let Ok(out) = commands::execute_captured("git", &["ls-files"]) {
            files = out.lines().map(|s| PathBuf::from(s.trim())).collect();
        }
    }

    if files.is_empty() {
        find_files_all(Path::new("."), &mut files);
    }

    let mut violations = 0;
    for file_path in files {
        if !file_path.is_file() {
            continue;
        }
        let file_str = file_path.to_string_lossy();
        if exclude_re.is_match(&file_str) {
            continue;
        }

        let file = File::open(&file_path).map_err(|e| XtaskError::InvalidConfig {
            message: e.to_string(),
        })?;
        let mut handle = file.take(1_048_576);
        let mut content = String::new();
        if handle.read_to_string(&mut content).is_ok() {
            for line in content.lines() {
                if email_re.is_match(line) && !exclude_re.is_match(line) {
                    println!("  ! Email detected in {}: {}", file_path.display(), line.trim());
                    violations += 1;
                }
            }
        }
    }

    if violations > 0 {
        return Err(XtaskError::InvalidConfig {
            message: format!("Privacy: {violations} email address(es) detected in codebase"),
        });
    }
    println!("  ✓ Privacy Check OK (No non-test emails found)");
    Ok(())
}

/// Runs secret scan checking for keys, tokens and passwords.
///
/// # Errors
/// Returns `XtaskError` if potential leak is found.
pub fn run_secret_scan() -> Result<(), XtaskError> {
    let secret_re = Regex::new(r#"(api_key|token|secret|password|auth|key)[[:space:]]*[:=][[:space:]]*['"][a-zA-Z0-9_-]{16,}['"]"#)
        .map_err(|e| XtaskError::InvalidConfig {
            message: e.to_string(),
        })?;

    let exclude_re = Regex::new(r"example\.com|example\.org|test\.com|GITHUB_TOKEN|CARGO_REGISTRY_TOKEN|worktree|\.git|target|\.cargo|\.agents|\.opencode|node_modules")
        .map_err(|e| XtaskError::InvalidConfig {
            message: e.to_string(),
        })?;

    let mut files = Vec::new();
    let is_git = Command::new("git")
        .arg("rev-parse")
        .arg("--git-dir")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if is_git {
        if let Ok(out) = commands::execute_captured("git", &["ls-files"]) {
            files = out.lines().map(|s| PathBuf::from(s.trim())).collect();
        }
    }

    if files.is_empty() {
        find_files_all(Path::new("."), &mut files);
    }

    let mut violations = 0;
    for file_path in files {
        if !file_path.is_file() {
            continue;
        }
        let file_str = file_path.to_string_lossy();
        if exclude_re.is_match(&file_str) {
            continue;
        }

        let file = File::open(&file_path).map_err(|e| XtaskError::InvalidConfig {
            message: e.to_string(),
        })?;
        let mut handle = file.take(1_048_576);
        let mut content = String::new();
        if handle.read_to_string(&mut content).is_ok() {
            for line in content.lines() {
                if secret_re.is_match(line) && !exclude_re.is_match(line) {
                    println!("  ! Secret pattern detected in {}: {}", file_path.display(), line.trim());
                    violations += 1;
                }
            }
        }
    }

    if violations > 0 {
        return Err(XtaskError::InvalidConfig {
            message: format!("Secret Scan: {violations} potential secret(s) detected in codebase"),
        });
    }
    println!("  ✓ Secret Scan OK (No potential secrets found)");
    Ok(())
}
