use rust_2026_template::add;

#[test]
fn snapshot_add_canonical_outputs() {
    let cases: Vec<(u64, u64, u64)> = vec![
        (0, 0, add(0, 0)),
        (1, 1, add(1, 1)),
        (10, 20, add(10, 20)),
        (100, 200, add(100, 200)),
        (u64::MAX, 0, add(u64::MAX, 0)),
    ];

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn snapshot_workspace_crate_names() {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("failed to run cargo metadata");

    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("failed to parse cargo metadata JSON");

    let mut crate_names: Vec<String> = metadata["packages"]
        .as_array()
        .expect("packages is not an array")
        .iter()
        .map(|pkg| {
            pkg["name"]
                .as_str()
                .expect("package name is not a string")
                .to_string()
        })
        .collect();

    crate_names.sort();

    insta::assert_yaml_snapshot!("workspace_crate_names", crate_names);
}
