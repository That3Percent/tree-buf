use crate::prelude::*;

#[cfg(feature = "encode")]
pub fn encode_packed_bool(items: &[bool], bytes: &mut Vec<u8>) {
    profile_fn!(encode_packed_bool);

    let mut offset = 0;
    while offset + 7 < items.len() {
        let b = u8::from(items[offset + 0]) << 0
            | u8::from(items[offset + 1]) << 1
            | u8::from(items[offset + 2]) << 2
            | u8::from(items[offset + 3]) << 3
            | u8::from(items[offset + 4]) << 4
            | u8::from(items[offset + 5]) << 5
            | u8::from(items[offset + 6]) << 6
            | u8::from(items[offset + 7]) << 7;
        bytes.push(b);
        offset += 8;
    }

    if offset < items.len() {
        let mut b = 0;
        for i in 0..items.len() - offset {
            b |= u8::from(items[offset + i]) << i;
        }
        bytes.push(b);
    }
}

#[cfg(feature = "decode")]
pub fn decode_packed_bool(bytes: &[u8]) -> Vec<bool> {
    profile_fn!(decode_packed_bool);

    let capacity = bytes.len() * 8;
    let mut result = Vec::with_capacity(capacity);
    for byte in bytes {
        result.extend_from_slice(&[
            (byte & 1 << 0) != 0,
            (byte & 1 << 1) != 0,
            (byte & 1 << 2) != 0,
            (byte & 1 << 3) != 0,
            (byte & 1 << 4) != 0,
            (byte & 1 << 5) != 0,
            (byte & 1 << 6) != 0,
            (byte & 1 << 7) != 0,
        ]);
    }
    debug_assert!(result.len() == capacity);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "decode", feature = "encode"))]
    #[test]
    fn round_trip_packed_bool() {
        let cases = vec![
            vec![],
            vec![true],
            vec![false],
            vec![true, true, true, true, true, true, true],
            vec![true, true, true, true, true, true, true, true],
            vec![true, true, true, true, true, true, true, true, true],
            vec![true, false, true, false, true, false, true, false, true, false, true, false, true, false, true, false],
            vec![false, true, false, true, false, true, false, true, false, true, false, true, false, true, false, true],
            vec![true, false, true, false, true, false, true, false, true, false, true, false, true, false, true, true, true],
            vec![
                false, true, false, true, false, true, false, true, false, true, false, true, false, true, false, false, false,
            ],
        ];

        for case in cases {
            let mut bytes = Vec::new();
            encode_packed_bool(&case, &mut bytes);
            let result = decode_packed_bool(&bytes);

            // Can't simply assert_eq, because the decoder will pad with false at the end.
            for i in 0..case.len() {
                assert_eq!(case[i], result[i]);
            }
        }
    }
}
