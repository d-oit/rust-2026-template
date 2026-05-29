use criterion::{Criterion, criterion_group, criterion_main};

fn bench_memory_usage(c: &mut Criterion) {
    c.bench_function("memory_usage_placeholder", |b| {
        b.iter(|| std::hint::black_box(vec![1u8; 1024]))
    });
}

criterion_group!(benches, bench_memory_usage);
criterion_main!(benches);
