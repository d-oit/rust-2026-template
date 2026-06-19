pub fn complex_function(x: i32) -> i32 {
    if x > 0 {
        if x < 10 {
            return x + 1;
        } else {
            return x - 1;
        }
    } else {
        return 0;
    }
}

pub fn manual_swap(a: &mut i32, b: &mut i32) {
    let temp = *a;
    *a = *b;
    *b = temp;
}
