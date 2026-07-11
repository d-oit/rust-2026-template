#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};

fn bench_greet(c: &mut Criterion) {
    let mut group = c.benchmark_group("greet");
    for name in ["a", "hello", "a longer name for testing"] {
        group.bench_function(format!("len_{}", name.len()), |b| {
            b.iter(|| std::hint::black_box(example_crate::greet(name)));
        });
    }
    group.finish();
}

fn bench_config_load(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bench.json");
    let json = r#"{"app_name":"bench","log_level":"info","max_items":100}"#;
    std::fs::write(&path, json).unwrap();

    c.bench_function("load_config_default", |b| {
        b.iter(|| std::hint::black_box(sample_app::load_config(None).unwrap()));
    });

    c.bench_function("load_config_from_file", |b| {
        b.iter(|| std::hint::black_box(sample_app::load_config(Some(path.clone())).unwrap()));
    });
}

criterion_group!(benches, bench_greet, bench_config_load);
criterion_main!(benches);
