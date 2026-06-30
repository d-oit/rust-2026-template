#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use sample_app::{is_safe_char, sanitize_str};

fn old_sanitize_str(s: &str) -> String {
    s.chars()
        .map(|c| if is_safe_char(c) { c } else { '?' })
        .collect()
}

fn bench_sanitization(c: &mut Criterion) {
    let clean_ascii = "This is a clean ASCII string that should not need any changes.";
    let dirty_unicode = "This string has some \n unsafe \u{202e} characters \u{200b}!";

    let mut group = c.benchmark_group("sanitization");

    group.bench_function("old_clean_ascii", |b| {
        b.iter(|| old_sanitize_str(black_box(clean_ascii)))
    });

    group.bench_function("new_clean_ascii", |b| {
        b.iter(|| sanitize_str(black_box(clean_ascii)))
    });

    group.bench_function("old_dirty_unicode", |b| {
        b.iter(|| old_sanitize_str(black_box(dirty_unicode)))
    });

    group.bench_function("new_dirty_unicode", |b| {
        b.iter(|| sanitize_str(black_box(dirty_unicode)))
    });

    group.finish();
}

criterion_group!(benches, bench_sanitization);
criterion_main!(benches);
