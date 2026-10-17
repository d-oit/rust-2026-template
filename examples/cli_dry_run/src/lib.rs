//! Safe `--dry-run` CLI pattern example for costly external side effects.
//!
//! This crate demonstrates how to implement a CLI tool that triggers paid APIs or
//! irreversible remote operations while offering a safe `--dry-run` mode to preview
//! execution plans without making network calls or mutating remote state.

#![forbid(unsafe_code)]

use clap::Parser;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during CLI validation or plan execution.
#[derive(Debug, Error)]
pub enum CliError {
    /// Validation error when arguments or configuration are invalid.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Remote API failure during execution.
    #[error("API error: {0}")]
    Api(String),
}

/// Command-line arguments for the dry-run example CLI.
#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[command(author, version, about = "Dry-run CLI pattern example", long_about = None)]
pub struct CliArgs {
    /// Path or name of the input audio manifest/file to process.
    #[arg(short, long)]
    pub input: String,

    /// Number of audio tracks to synthesize or process (1..=100).
    #[arg(short, long, default_value_t = 1)]
    pub tracks: u32,

    /// Audio processing quality tier (`standard`, `hd`, or `lossless`).
    #[arg(short, long, default_value = "standard")]
    pub quality: String,

    /// Preview the execution plan and cost without invoking remote APIs or mutating state.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Individual action planned for execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedAction {
    /// Step index in the execution sequence.
    pub step: usize,
    /// Description of the action.
    pub description: String,
    /// Whether this step involves a costly remote API call.
    pub is_costly_remote: bool,
}

/// Fully validated execution plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// Input file or manifest identifier.
    pub input: String,
    /// Number of tracks to process.
    pub tracks_count: u32,
    /// Quality tier requested.
    pub quality: String,
    /// Estimated quota/credits required (based on deterministic tier pricing).
    pub estimated_credits: u64,
    /// Ordered sequence of planned actions.
    pub planned_actions: Vec<PlannedAction>,
    /// Whether dry-run mode was requested.
    pub dry_run: bool,
}

impl Plan {
    /// Builds a validated [`Plan`] from raw [`CliArgs`].
    ///
    /// Validation rules are applied uniformly before plan construction regardless
    /// of whether `dry_run` is set.
    ///
    /// # Errors
    ///
    /// Returns [`CliError::Validation`] if argument constraints are violated.
    pub fn build(args: &CliArgs) -> Result<Self, CliError> {
        let trimmed_input = args.input.trim();
        if trimmed_input.is_empty() {
            return Err(CliError::Validation("Input path cannot be empty".into()));
        }
        if trimmed_input.chars().any(char::is_control) {
            return Err(CliError::Validation(
                "Input path contains invalid control characters".into(),
            ));
        }

        if args.tracks == 0 || args.tracks > 100 {
            return Err(CliError::Validation(
                "Tracks count must be between 1 and 100".into(),
            ));
        }

        let per_track_cost = match args.quality.to_lowercase().as_str() {
            "standard" => 2,
            "hd" => 5,
            "lossless" => 10,
            _ => {
                return Err(CliError::Validation(format!(
                    "Invalid quality tier '{}'. Expected 'standard', 'hd', or 'lossless'",
                    args.quality
                )));
            }
        };

        let estimated_credits = u64::from(args.tracks) * per_track_cost;

        let mut planned_actions = Vec::new();
        planned_actions.push(PlannedAction {
            step: 1,
            description: format!("Validate local input manifest: '{trimmed_input}'"),
            is_costly_remote: false,
        });

        for i in 1..=args.tracks {
            planned_actions.push(PlannedAction {
                step: planned_actions.len() + 1,
                description: format!(
                    "Synthesize track #{i}/{}: tier '{}' (cost: {per_track_cost} credits)",
                    args.tracks, args.quality
                ),
                is_costly_remote: true,
            });
        }

        planned_actions.push(PlannedAction {
            step: planned_actions.len() + 1,
            description: format!("Finalize and store output batch for '{trimmed_input}'"),
            is_costly_remote: false,
        });

        Ok(Self {
            input: trimmed_input.to_string(),
            tracks_count: args.tracks,
            quality: args.quality.clone(),
            estimated_credits,
            planned_actions,
            dry_run: args.dry_run,
        })
    }
}

/// Abstract remote API client interface for external operations.
pub trait ApiClient {
    /// Executes a remote action against the paid external service.
    ///
    /// # Errors
    ///
    /// Returns [`CliError::Api`] if the remote operation fails.
    fn execute_remote(&self, step: usize, description: &str) -> Result<String, CliError>;
}

/// Result summary of plan execution or dry-run inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionReport {
    /// Whether actual execution took place (`false` for dry-run).
    pub executed: bool,
    /// Number of actions successfully performed.
    pub actions_performed: usize,
    /// Number of actions skipped (e.g. in dry-run mode).
    pub actions_skipped: usize,
    /// Human-readable log lines describing the outcome.
    pub log_output: Vec<String>,
}

/// Executes a plan using the provided API client.
///
/// If `plan.dry_run` is `true`, no calls are made to `client.execute_remote()`,
/// and external state is untouched.
///
/// # Errors
///
/// Returns [`CliError`] if remote execution fails during live mode.
pub fn run_plan(plan: &Plan, client: &impl ApiClient) -> Result<ExecutionReport, CliError> {
    let mut logs = Vec::new();

    if plan.dry_run {
        logs.push("[DRY-RUN] Mode active — inspecting execution plan without side effects.".into());
        logs.push(format!(
            "[DRY-RUN] Scope: {} track(s) at '{}' quality. Estimated credits: {}.",
            plan.tracks_count, plan.quality, plan.estimated_credits
        ));
        logs.push("[DRY-RUN] Actions that WOULD be performed:".into());

        let mut skipped_count = 0;
        for action in &plan.planned_actions {
            if action.is_costly_remote {
                skipped_count += 1;
                logs.push(format!(
                    "  [WOULD EXECUTE] Step {}: {} (SKIPPED paid API call)",
                    action.step, action.description
                ));
            } else {
                logs.push(format!(
                    "  [WOULD EXECUTE] Step {}: {}",
                    action.step, action.description
                ));
            }
        }

        logs.push(format!(
            "[DRY-RUN] Completed inspection: 0 actions performed, {skipped_count} paid remote call(s) skipped."
        ));

        Ok(ExecutionReport {
            executed: false,
            actions_performed: 0,
            actions_skipped: skipped_count,
            log_output: logs,
        })
    } else {
        logs.push(format!(
            "Starting live execution for input '{}' ({} tracks, {} quality, {} credits)...",
            plan.input, plan.tracks_count, plan.quality, plan.estimated_credits
        ));

        let mut performed = 0;
        for action in &plan.planned_actions {
            if action.is_costly_remote {
                let resp = client.execute_remote(action.step, &action.description)?;
                logs.push(format!("  Step {}: {resp}", action.step));
                performed += 1;
            } else {
                logs.push(format!("  Step {}: Local step completed", action.step));
                performed += 1;
            }
        }

        logs.push("Live execution completed successfully.".into());

        Ok(ExecutionReport {
            executed: true,
            actions_performed: performed,
            actions_skipped: 0,
            log_output: logs,
        })
    }
}
