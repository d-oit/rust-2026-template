//! Template initialization logic (profile-driven, issue #286).
//!
//! A `cargo xtask template init --profile <id>` loads the matching blueprint
//! from `config/template-profiles/`, builds a fully validated [`InitPlan`]
//! (all reads, path containment checks, and content preparation — zero
//! mutation), then applies it atomically: removals, rename, rewrites.
//!
//! [`InitPlan`]: plan::InitPlan

pub(crate) mod apply;
pub(crate) mod plan;
pub(crate) mod validate;

use crate::config::XtaskError;
use crate::template_profile::TemplateProfile;

/// Run the template initialization using a validated profile blueprint.
///
/// Planning is total: by the time any file is touched, every operation has
/// been decided and every target validated. Dry runs print the same plan the
/// real run would execute.
///
/// # Errors
/// Returns `XtaskError` if the profile/identity is invalid, any target path
/// escapes the repository, or a filesystem operation fails.
pub fn run_init(
    profile: &str,
    name: Option<&str>,
    description: Option<&str>,
    author: Option<&str>,
    repo: Option<&str>,
    dry_run: bool,
) -> Result<(), XtaskError> {
    let blueprint = TemplateProfile::load(profile)?;
    println!(
        "==> Initializing template with profile: {}",
        blueprint.metadata.id
    );

    let identity = validate::ProjectIdentity::new(name, description, author, repo)?;
    let root = std::env::current_dir().map_err(|e| XtaskError::InvalidConfig {
        message: format!("cannot determine repository root (cwd): {e}"),
    })?;

    let plan = plan::InitPlan::build(&root, &blueprint, &identity)?;
    if dry_run {
        plan.print_dry_run();
        return Ok(());
    }

    apply::execute(&plan)?;
    println!(
        "  ✓ Template initialized successfully with profile '{}'!",
        blueprint.metadata.id
    );
    Ok(())
}
