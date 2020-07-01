use crate::prelude::*;
use std::ops::{Add, Sub};

// FIXME: This may not be what is needed. Zigzag may be.
pub fn delta_encode_in_place<T: Sub<Output = T> + Copy>(data: &mut [T]) {
    profile_fn!(delta_encode_in_place);
    if data.len() == 0 {
        return;
    }
    let mut current = data[0];
    for i in 1..data.len() {
        let next = data[i];
        data[i] = next - current;
        current = next;
    }
}

pub fn delta_decode_in_place<T: Add<Output = T> + Copy>(data: &mut [T]) {
    profile_fn!(delta_decode_in_place);
    for i in 1..data.len() {
        data[i] = data[i] + data[i - 1];
    }
}

#[cfg(tests)]
mod tests {
    use super::*;
    #[test]
    pub fn round_trip() {
        let tests = vec![vec![], vec![10], vec![0, 1]];
        for test in tests {}
    }
}
