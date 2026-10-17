use cli_dry_run::{ApiClient, CliArgs, CliError, Plan, run_plan};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A mock client that records calls and fails if called when unexpected.
struct MockApiClient {
    call_count: AtomicUsize,
    should_panic: bool,
}

impl MockApiClient {
    fn new_strict() -> Self {
        Self {
            call_count: AtomicUsize::new(0),
            should_panic: true,
        }
    }

    fn new_permissive() -> Self {
        Self {
            call_count: AtomicUsize::new(0),
            should_panic: false,
        }
    }

    fn calls(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

impl ApiClient for MockApiClient {
    fn execute_remote(&self, step: usize, description: &str) -> Result<String, CliError> {
        if self.should_panic {
            panic!("OUTBOUND REQUEST TRIGGERED DURING DRY-RUN! Step: {step}, Desc: {description}");
        }
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(format!("Mock response for step {step}"))
    }
}

#[test]
fn test_dry_run_makes_no_outbound_request() {
    let args = CliArgs {
        input: "album_manifest.json".into(),
        tracks: 5,
        quality: "hd".into(),
        dry_run: true,
    };

    let plan = Plan::build(&args).expect("Plan construction should succeed");
    assert!(plan.dry_run);

    // Client configured to panic on any remote API call
    let client = MockApiClient::new_strict();

    let report = run_plan(&plan, &client).expect("run_plan should succeed in dry-run");

    assert!(!report.executed);
    assert_eq!(report.actions_performed, 0);
    assert_eq!(report.actions_skipped, 5);
    assert_eq!(client.calls(), 0);

    // Verify output clearly identifies planned actions without claiming execution
    let combined_logs = report.log_output.join("\n");
    assert!(combined_logs.contains("[DRY-RUN]"));
    assert!(combined_logs.contains("[WOULD EXECUTE]"));
    assert!(combined_logs.contains("SKIPPED paid API call"));
    assert!(combined_logs.contains("0 actions performed"));
    assert!(!combined_logs.contains("Live execution completed"));
}

#[test]
fn test_shared_validation_logic_between_dry_run_and_execution() {
    // 1. Empty input validation
    let invalid_args_dry_run = CliArgs {
        input: "   ".into(),
        tracks: 3,
        quality: "standard".into(),
        dry_run: true,
    };
    let invalid_args_live = CliArgs {
        input: "   ".into(),
        tracks: 3,
        quality: "standard".into(),
        dry_run: false,
    };

    let err_dry_run = Plan::build(&invalid_args_dry_run).unwrap_err();
    let err_live = Plan::build(&invalid_args_live).unwrap_err();

    assert_eq!(err_dry_run.to_string(), err_live.to_string());
    assert!(
        err_dry_run
            .to_string()
            .contains("Input path cannot be empty")
    );

    // 2. Invalid track count
    let invalid_tracks_dry_run = CliArgs {
        input: "valid.json".into(),
        tracks: 0,
        quality: "standard".into(),
        dry_run: true,
    };
    let invalid_tracks_live = CliArgs {
        input: "valid.json".into(),
        tracks: 0,
        quality: "standard".into(),
        dry_run: false,
    };

    let err_tracks_dry_run = Plan::build(&invalid_tracks_dry_run).unwrap_err();
    let err_tracks_live = Plan::build(&invalid_tracks_live).unwrap_err();

    assert_eq!(err_tracks_dry_run.to_string(), err_tracks_live.to_string());
    assert!(
        err_tracks_dry_run
            .to_string()
            .contains("Tracks count must be between 1 and 100")
    );

    // 3. Unknown quality tier
    let invalid_quality_dry_run = CliArgs {
        input: "valid.json".into(),
        tracks: 2,
        quality: "ultra-mega".into(),
        dry_run: true,
    };
    let invalid_quality_live = CliArgs {
        input: "valid.json".into(),
        tracks: 2,
        quality: "ultra-mega".into(),
        dry_run: false,
    };

    let err_qual_dry_run = Plan::build(&invalid_quality_dry_run).unwrap_err();
    let err_qual_live = Plan::build(&invalid_quality_live).unwrap_err();

    assert_eq!(err_qual_dry_run.to_string(), err_qual_live.to_string());
    assert!(
        err_qual_dry_run
            .to_string()
            .contains("Invalid quality tier 'ultra-mega'")
    );
}

#[test]
fn test_live_execution_calls_remote_api() {
    let args = CliArgs {
        input: "album_manifest.json".into(),
        tracks: 2,
        quality: "lossless".into(),
        dry_run: false,
    };

    let plan = Plan::build(&args).expect("Plan construction should succeed");
    assert!(!plan.dry_run);
    assert_eq!(plan.estimated_credits, 20); // 2 tracks * 10 credits

    let client = MockApiClient::new_permissive();

    let report = run_plan(&plan, &client).expect("run_plan should succeed in live mode");

    assert!(report.executed);
    // Total actions performed = 1 local validation + 2 remote synthesize + 1 local finalize = 4
    assert_eq!(report.actions_performed, 4);
    assert_eq!(report.actions_skipped, 0);
    assert_eq!(client.calls(), 2);

    let combined_logs = report.log_output.join("\n");
    assert!(combined_logs.contains("Starting live execution"));
    assert!(combined_logs.contains("Live execution completed successfully"));
}
