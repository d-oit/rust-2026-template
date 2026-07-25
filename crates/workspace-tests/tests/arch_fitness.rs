#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::collections::HashMap;

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

/// Returns the architectural layer for a crate based on its name suffix.
fn crate_layer(name: &str) -> Option<u8> {
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

#[test]
fn crate_layering_no_upward_dependencies() {
    let dep_map = workspace_dep_map();

    for (crate_name, deps) in &dep_map {
        if let Some(layer) = crate_layer(crate_name) {
            for dep in deps {
                if let Some(dep_layer) = crate_layer(dep) {
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
        }
    }
}
