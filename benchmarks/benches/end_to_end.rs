use criterion::{Criterion, criterion_group, criterion_main};

fn bench_end_to_end(c: &mut Criterion) {
    c.bench_function("end_to_end_placeholder", |b| {
        b.iter(|| std::hint::black_box(42))
    });
}

criterion_group!(benches, bench_end_to_end);
criterion_main!(benches);
