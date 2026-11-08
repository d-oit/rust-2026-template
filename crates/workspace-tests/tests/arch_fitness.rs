#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    reason = "test modules assert on known-good fixtures, so panicking macros are the intended failure signal"
)]

use std::collections::{BTreeMap, HashMap};

/// Parses `cargo metadata` and returns a map of crate names to their workspace dependency names.
fn workspace_dep_map() -> HashMap<String, Vec<String>> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("failed to run cargo metadata");

    let mut map = HashMap::new();
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("failed to parse cargo metadata JSON");

    if let Some(packages) = metadata["packages"].as_array() {
        for pkg in packages {
            let name = pkg["name"].as_str().unwrap_or("").to_string();
            let deps: Vec<String> = pkg["dependencies"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|d| d["name"].as_str().map(String::from))
                .collect();
            map.insert(name, deps);
        }
    }

    map
}

/// Crates intentionally outside the layer model, with the reason they are exempt.
///
/// Only developer tooling that is never linked into a shipped artifact belongs
/// here. Applying the layering rule to it would be meaningless, not permissive.
const EXEMPT: &[(&str, &str)] = &[(
    "xtask",
    "developer tooling binary; never linked into a shipped artifact",
)];

/// Explicit layer assignment for every crate that ships with this template.
///
/// Layer 0 is the most foundational; a crate may depend only on crates in an
/// equal or lower layer.
const CLASSIFIED: &[(&str, u8)] = &[
    // Layer 0 — shared primitives with no workspace dependencies.
    ("example-crate", 0),
    // Layer 1 — traits and domain logic; zero concrete backends.
    ("example-storage-pattern", 1),
    ("example-registry-pattern", 1),
    ("actor-runtime-template", 1),
    // Layer 2 — concrete storage/transport implementations.
    ("checkpoint-template", 2),
    ("hybrid-storage-template", 2),
    ("mcp-server-template", 2),
    // Layer 3 — applications, examples, and test/benchmark harnesses that
    // compose the layers below them.
    ("sample-app", 3),
    ("hello_world", 3),
    ("cli_dry_run", 3),
    ("security-egress-and-path-validation", 3),
    ("security_secret_detector", 3),
    ("benchmarks", 3),
    ("workspace-tests", 3),
];

/// Returns the architectural layer for a crate, or `None` when the crate is not
/// covered by the layer model.
///
/// [`CLASSIFIED`] is authoritative because this template's crates do not follow
/// one naming scheme. The suffix rules let adopters opt in simply by naming
/// crates (`*-types`, `*-core`, `*-adapters`, `*-app`).
fn crate_layer(name: &str) -> Option<u8> {
    if let Some((_, layer)) = CLASSIFIED
        .iter()
        .find(|(crate_name, _)| *crate_name == name)
    {
        return Some(*layer);
    }
    if name.ends_with("-types") || name.ends_with("-domain") {
        Some(0)
    } else if name.ends_with("-core") || name.ends_with("-logic") {
        Some(1)
    } else if name.ends_with("-adapters")
        || name.ends_with("-backends")
        || name.ends_with("-adapter")
    {
        Some(2)
    } else if name.ends_with("-cli") || name.ends_with("-bin") || name.ends_with("-app") {
        Some(3)
    } else {
        None
    }
}

/// Reports how the layer model treats every workspace member.
fn classification_map(dep_map: &HashMap<String, Vec<String>>) -> BTreeMap<String, String> {
    dep_map
        .keys()
        .map(|name| {
            let treatment = crate_layer(name).map_or_else(
                || {
                    EXEMPT
                        .iter()
                        .find(|(crate_name, _)| crate_name == name)
                        .map_or_else(
                            || "UNCLASSIFIED".to_string(),
                            |(_, reason)| format!("exempt ({reason})"),
                        )
                },
                |layer| format!("layer {layer}"),
            );
            (name.clone(), treatment)
        })
        .collect()
}

#[test]
fn crate_layering_no_upward_dependencies() {
    let dep_map = workspace_dep_map();

    let mut checked_pairs = Vec::new();
    for (crate_name, deps) in &dep_map {
        let Some(layer) = crate_layer(crate_name) else {
            continue;
        };
        for dep in deps {
            let Some(dep_layer) = crate_layer(dep) else {
                continue;
            };
            checked_pairs.push(format!("{crate_name}->{dep}"));
            assert!(
                dep_layer <= layer,
                "HARNESS VIOLATION: Crate `{crate_name}` (layer {layer}) \
                 depends on `{dep}` (layer {dep_layer}). \
                 Upward dependencies are not allowed. \
                 FIX: Move shared types to a lower-layer crate, \
                 introduce a trait in the lower layer, or restructure the dependency graph."
            );
        }
    }

    // A layering check that inspects zero edges is a false pass, not a guardrail.
    assert!(
        checked_pairs.len() >= 5,
        "HARNESS VIOLATION: the layering check inspected only {} intra-workspace \
         edge(s) ({checked_pairs:?}), so it cannot meaningfully fail. \
         FIX: classify the workspace crates in `CLASSIFIED` so real dependencies \
         are verified.",
        checked_pairs.len()
    );
}

/// Every workspace member must be deliberately classified or explicitly exempt.
///
/// Without this, adding a new crate silently escapes the layer model — which is
/// how the original name-suffix-only rule ended up matching nothing at all.
#[test]
fn every_workspace_crate_is_classified_or_exempt() {
    let dep_map = workspace_dep_map();
    let unclassified: Vec<_> = classification_map(&dep_map)
        .into_iter()
        .filter(|(_, treatment)| treatment == "UNCLASSIFIED")
        .map(|(name, _)| name)
        .collect();

    assert!(
        unclassified.is_empty(),
        "HARNESS VIOLATION: crates not covered by the architecture model: {unclassified:?}. \
         FIX: add each one to `CLASSIFIED` with its layer, or to `EXEMPT` with a reason."
    );
}

/// Entries in `CLASSIFIED` and `EXEMPT` must name crates that still exist, so
/// stale entries cannot accumulate after a rename or removal.
#[test]
fn model_entries_reference_existing_crates() {
    let dep_map = workspace_dep_map();
    let stale: Vec<_> = CLASSIFIED
        .iter()
        .map(|(name, _)| *name)
        .chain(EXEMPT.iter().map(|(name, _)| *name))
        .filter(|name| !dep_map.contains_key(*name))
        .collect();

    assert!(
        stale.is_empty(),
        "HARNESS VIOLATION: the architecture model lists crates that are no longer \
         workspace members: {stale:?}. FIX: remove or rename those entries."
    );
}
