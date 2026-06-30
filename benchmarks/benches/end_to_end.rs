#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};

fn bench_add(c: &mut Criterion) {
    c.bench_function("add_basic", |b| {
        b.iter(|| std::hint::black_box(rust_2026_template::add(2, 3)))
    });
}

fn bench_process_items(c: &mut Criterion) {
    let mut group = c.benchmark_group("process_items");
    for count in [10, 100, 1000] {
        group.bench_function(format!("count_{count}"), |b| {
            b.iter(|| std::hint::black_box(sample_app::process_items(count, 10000).unwrap()))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_add, bench_process_items);
criterion_main!(benches);
