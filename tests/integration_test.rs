use rust_2026_template::add;

#[test]
fn integration_add_works() {
    assert_eq!(add(10, 20), 30);
}

#[test]
fn integration_add_negative_numbers() {
    assert_eq!(add(5, 0), 5);
    assert_eq!(add(0, 5), 5);
}

#[test]
fn integration_add_large_numbers() {
    assert_eq!(add(u64::MAX, 0), u64::MAX);
    assert_eq!(add(0, u64::MAX), u64::MAX);
}

#[test]
fn integration_add_symmetry() {
    for a in 0..=100u64 {
        for b in 0..=100u64 {
            assert_eq!(add(a, b), add(b, a), "add({a}, {b}) != add({b}, {a})");
        }
    }
}
