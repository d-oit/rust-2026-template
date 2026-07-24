#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn is_invalid_app_name_current(app_name: &str) -> bool {
    app_name.chars().any(|c| {
        c.is_control()
            || matches!(
                c,
                '\u{200b}'..='\u{200f}'
                    | '\u{202a}'..='\u{202e}'
                    | '\u{2066}'..='\u{2069}'
            )
    })
}

fn is_invalid_app_name_new(app_name: &str) -> bool {
    let bytes = app_name.as_bytes();
    let chunks = bytes.chunks_exact(8);
    let mut chunk_offset = 0;
    for chunk_bytes in chunks {
        let chunk = u64::from_ne_bytes(chunk_bytes.try_into().unwrap());
        let low_check = (chunk.wrapping_sub(0x2020202020202020) & !chunk) & 0x8080808080808080;
        let high_check = (chunk | chunk.wrapping_add(0x0101010101010101)) & 0x8080808080808080;
        if (low_check | high_check) != 0 {
            break;
        }
        chunk_offset += 8;
    }

    let mut i = chunk_offset;
    while i < bytes.len() {
        let b = bytes[i];
        if !(0x20..=0x7E).contains(&b) {
            break;
        }
        i += 1;
    }

    if i == bytes.len() {
        false
    } else {
        app_name[i..].chars().any(|c| {
            c.is_control()
                || matches!(
                    c,
                    '\u{200b}'..='\u{200f}'
                        | '\u{202a}'..='\u{202e}'
                        | '\u{2066}'..='\u{2069}'
                )
        })
    }
}

fn bench_checkpoint_validation(c: &mut Criterion) {
    let clean_ascii = "my-awesome-app-2026";
    let dirty_unicode = "my-awesome-app\u{202e}-2026";

    let mut group = c.benchmark_group("checkpoint_validation");

    group.bench_function("current_clean_ascii", |b| {
        b.iter(|| is_invalid_app_name_current(black_box(clean_ascii)));
    });

    group.bench_function("new_clean_ascii", |b| {
        b.iter(|| is_invalid_app_name_new(black_box(clean_ascii)));
    });

    group.bench_function("current_dirty_unicode", |b| {
        b.iter(|| is_invalid_app_name_current(black_box(dirty_unicode)));
    });

    group.bench_function("new_dirty_unicode", |b| {
        b.iter(|| is_invalid_app_name_new(black_box(dirty_unicode)));
    });

    group.finish();
}

criterion_group!(benches, bench_checkpoint_validation);
criterion_main!(benches);
