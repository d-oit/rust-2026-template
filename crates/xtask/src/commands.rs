//! Safe process execution wrapper.
#![allow(clippy::unwrap_used)]

use crate::config::XtaskError;
use std::process::Command;

/// Resolves program name for Windows compatibility.
#[must_use]
pub fn resolve_program(program: &str) -> String {
    if std::env::consts::OS == "windows" && program == "markdownlint-cli2" {
        return format!("{program}.cmd");
    }
    program.to_string()
}

/// Safely execute a command without shell interpretation.
/// Captures status and returns `XtaskError::CommandFailure` if not successful.
///
/// # Errors
/// Returns `XtaskError::CommandFailure` if the command fails to spawn or returns non-zero.
pub fn execute(program: &str, args: &[&str]) -> Result<(), XtaskError> {
    let resolved = resolve_program(program);
    println!("  → {resolved} {}", args.join(" "));
    let status = Command::new(&resolved)
        .args(args)
        .status()
        .map_err(|_e| XtaskError::CommandFailure {
            command: format!("{resolved} {}", args.join(" ")),
            exit_code: None,
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(XtaskError::CommandFailure {
            command: format!("{resolved} {}", args.join(" ")),
            exit_code: status.code(),
        })
    }
}

/// Safely execute a command and capture its stdout as a String.
///
/// # Errors
/// Returns `XtaskError::CommandFailure` if the command fails to spawn or returns non-zero.
pub fn execute_captured(program: &str, args: &[&str]) -> Result<String, XtaskError> {
    let resolved = resolve_program(program);
    let output = Command::new(&resolved)
        .args(args)
        .output()
        .map_err(|_e| XtaskError::CommandFailure {
            command: format!("{resolved} {}", args.join(" ")),
            exit_code: None,
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(XtaskError::CommandFailure {
            command: format!("{resolved} {}", args.join(" ")),
            exit_code: output.status.code(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_program() {
        let prog = "markdownlint-cli2";
        let resolved = resolve_program(prog);
        if std::env::consts::OS == "windows" {
            assert_eq!(resolved, "markdownlint-cli2.cmd");
        } else {
            assert_eq!(resolved, "markdownlint-cli2");
        }
    }

    #[test]
    fn test_execute_echo() {
        // Ensure running a simple standard command works
        let res = execute("git", &["--version"]);
        assert!(res.is_ok());
    }

    #[test]
    fn test_execute_captured() {
        let res = execute_captured("git", &["--version"]);
        assert!(res.is_ok());
        assert!(res.unwrap().contains("git version"));
    }

    #[test]
    fn test_execute_fail() {
        let res = execute("git", &["non-existent-subcommand"]);
        assert!(res.is_err());
    }
}
