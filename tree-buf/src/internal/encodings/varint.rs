use crate::prelude::*;

#[cfg(feature = "encode")]
pub fn size_for_varint(value: u64) -> usize {
    if value < (1 << 7) {
        1
    } else if value < (1 << 14) {
        2
    } else if value < (1 << 21) {
        3
    } else if value < (1 << 28) {
        4
    } else if value < (1 << 35) {
        5
    } else if value < (1 << 42) {
        6
    } else if value < (1 << 49) {
        7
    } else if value < (1 << 56) {
        8
    } else {
        9
    }
}

#[cfg(feature = "encode")]
pub fn encode_prefix_varint(value: u64, into: &mut Vec<u8>) {
    if value < (1 << 7) {
        into.push((value << 1) as u8 | 1);
    } else if value < (1 << 14) {
        into.extend_from_slice(&[(value << 2) as u8 | (1 << 1), (value >> 6) as u8]);
    } else if value < (1 << 21) {
        into.extend_from_slice(&[(value << 3) as u8 | (1 << 2), (value >> 5) as u8, (value >> 13) as u8]);
    } else if value < (1 << 28) {
        into.extend_from_slice(&[(value << 4) as u8 | (1 << 3), (value >> 4) as u8, (value >> 12) as u8, (value >> 20) as u8]);
    } else if value < (1 << 35) {
        into.extend_from_slice(&[
            (value << 5) as u8 | (1 << 4),
            (value >> 3) as u8,
            (value >> 11) as u8,
            (value >> 19) as u8,
            (value >> 27) as u8,
        ]);
    } else if value < (1 << 42) {
        into.extend_from_slice(&[
            (value << 6) as u8 | (1 << 5),
            (value >> 2) as u8,
            (value >> 10) as u8,
            (value >> 18) as u8,
            (value >> 26) as u8,
            (value >> 34) as u8,
        ]);
    } else if value < (1 << 49) {
        into.extend_from_slice(&[
            (value << 7) as u8 | (1 << 6),
            (value >> 1) as u8,
            (value >> 9) as u8,
            (value >> 17) as u8,
            (value >> 25) as u8,
            (value >> 33) as u8,
            (value >> 41) as u8,
        ]);
    } else if value < (1 << 56) {
        into.extend_from_slice(&[
            (1 << 7),
            value as u8,
            (value >> 8) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
            (value >> 32) as u8,
            (value >> 40) as u8,
            (value >> 48) as u8,
        ]);
    } else {
        into.extend_from_slice(&[
            0,
            value as u8,
            (value >> 8) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
            (value >> 32) as u8,
            (value >> 40) as u8,
            (value >> 48) as u8,
            (value >> 56) as u8,
        ]);
    }
}

/// This is much like prefix varint, but with the tag bits in the last byte.
/// Useful for reading backwards.
#[cfg(feature = "encode")]
pub fn encode_suffix_varint(value: u64, into: &mut Vec<u8>) {
    if value < (1 << 7) {
        into.push((value << 1) as u8 | 1);
    } else if value < (1 << 14) {
        into.extend_from_slice(&[(value >> 6) as u8, (value << 2) as u8 | (1 << 1)]);
    } else if value < (1 << 21) {
        into.extend_from_slice(&[(value >> 5) as u8, (value >> 13) as u8, (value << 3) as u8 | (1 << 2)]);
    } else if value < (1 << 28) {
        into.extend_from_slice(&[(value >> 4) as u8, (value >> 12) as u8, (value >> 20) as u8, (value << 4) as u8 | (1 << 3)]);
    } else if value < (1 << 35) {
        into.extend_from_slice(&[
            (value >> 3) as u8,
            (value >> 11) as u8,
            (value >> 19) as u8,
            (value >> 27) as u8,
            (value << 5) as u8 | (1 << 4),
        ]);
    } else if value < (1 << 42) {
        into.extend_from_slice(&[
            (value >> 2) as u8,
            (value >> 10) as u8,
            (value >> 18) as u8,
            (value >> 26) as u8,
            (value >> 34) as u8,
            (value << 6) as u8 | (1 << 5),
        ]);
    } else if value < (1 << 49) {
        into.extend_from_slice(&[
            (value >> 1) as u8,
            (value >> 9) as u8,
            (value >> 17) as u8,
            (value >> 25) as u8,
            (value >> 33) as u8,
            (value >> 41) as u8,
            (value << 7) as u8 | (1 << 6),
        ]);
    } else if value < (1 << 56) {
        into.extend_from_slice(&[
            value as u8,
            (value >> 8) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
            (value >> 32) as u8,
            (value >> 40) as u8,
            (value >> 48) as u8,
            (1 << 7),
        ]);
    } else {
        into.extend_from_slice(&[
            value as u8,
            (value >> 8) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
            (value >> 32) as u8,
            (value >> 40) as u8,
            (value >> 48) as u8,
            (value >> 56) as u8,
            0,
        ]);
    }
}

#[cfg(feature = "decode")]
pub fn decode_prefix_varint(bytes: &[u8], offset: &mut usize) -> DecodeResult<u64> {
    let first = bytes.get(*offset).ok_or_else(|| DecodeError::InvalidFormat)?;
    let shift = first.trailing_zeros();

    // TODO: Check that the compiler does unchecked indexing after this
    if (*offset + (shift as usize)) >= bytes.len() {
        return Err(DecodeError::InvalidFormat);
    }

    let result = match shift {
        0 => (first >> 1) as u64,
        1 => (first >> 2) as u64 | ((bytes[*offset + 1] as u64) << 6),
        2 => (first >> 3) as u64 | ((bytes[*offset + 1] as u64) << 5) | ((bytes[*offset + 2] as u64) << 13),
        3 => (first >> 4) as u64 | ((bytes[*offset + 1] as u64) << 4) | ((bytes[*offset + 2] as u64) << 12) | ((bytes[*offset + 3] as u64) << 20),
        4 => {
            (first >> 5) as u64
                | ((bytes[*offset + 1] as u64) << 3)
                | ((bytes[*offset + 2] as u64) << 11)
                | ((bytes[*offset + 3] as u64) << 19)
                | ((bytes[*offset + 4] as u64) << 27)
        }
        5 => {
            (first >> 6) as u64
                | ((bytes[*offset + 1] as u64) << 2)
                | ((bytes[*offset + 2] as u64) << 10)
                | ((bytes[*offset + 3] as u64) << 18)
                | ((bytes[*offset + 4] as u64) << 26)
                | ((bytes[*offset + 5] as u64) << 34)
        }
        6 => {
            (first >> 7) as u64
                | ((bytes[*offset + 1] as u64) << 1)
                | ((bytes[*offset + 2] as u64) << 9)
                | ((bytes[*offset + 3] as u64) << 17)
                | ((bytes[*offset + 4] as u64) << 25)
                | ((bytes[*offset + 5] as u64) << 33)
                | ((bytes[*offset + 6] as u64) << 41)
        }
        7 => {
            (bytes[*offset + 1] as u64)
                | ((bytes[*offset + 2] as u64) << 8)
                | ((bytes[*offset + 3] as u64) << 16)
                | ((bytes[*offset + 4] as u64) << 24)
                | ((bytes[*offset + 5] as u64) << 32)
                | ((bytes[*offset + 6] as u64) << 40)
                | ((bytes[*offset + 7] as u64) << 48)
        }
        8 => {
            (bytes[*offset + 1] as u64)
                | ((bytes[*offset + 2] as u64) << 8)
                | ((bytes[*offset + 3] as u64) << 16)
                | ((bytes[*offset + 4] as u64) << 24)
                | ((bytes[*offset + 5] as u64) << 32)
                | ((bytes[*offset + 6] as u64) << 40)
                | ((bytes[*offset + 7] as u64) << 48)
                | ((bytes[*offset + 8] as u64) << 56)
        }
        _ => unreachable!(),
    };
    *offset += (shift + 1) as usize;
    Ok(result)
}

/// Because this reads backwards, beware that the offset will end up at std::usize::MAX if the first byte is read past.
#[cfg(feature = "decode")]
pub fn decode_suffix_varint(bytes: &[u8], offset: &mut usize) -> DecodeResult<u64> {
    let first = bytes.get(*offset).ok_or_else(|| DecodeError::InvalidFormat)?;
    let shift = first.trailing_zeros();

    // TODO: Ensure unchecked indexing follows.
    if *offset < (shift as usize) {
        return Err(DecodeError::InvalidFormat);
    }

    let result = match shift {
        0 => (first >> 1) as u64,
        1 => (first >> 2) as u64 | ((bytes[*offset - 1] as u64) << 6),
        2 => (first >> 3) as u64 | ((bytes[*offset - 2] as u64) << 5) | ((bytes[*offset - 1] as u64) << 13),
        3 => (first >> 4) as u64 | ((bytes[*offset - 3] as u64) << 4) | ((bytes[*offset - 2] as u64) << 12) | ((bytes[*offset - 1] as u64) << 20),
        4 => {
            (first >> 5) as u64
                | ((bytes[*offset - 4] as u64) << 3)
                | ((bytes[*offset - 3] as u64) << 11)
                | ((bytes[*offset - 2] as u64) << 19)
                | ((bytes[*offset - 1] as u64) << 27)
        }
        5 => {
            (first >> 6) as u64
                | ((bytes[*offset - 5] as u64) << 2)
                | ((bytes[*offset - 4] as u64) << 10)
                | ((bytes[*offset - 3] as u64) << 18)
                | ((bytes[*offset - 2] as u64) << 26)
                | ((bytes[*offset - 1] as u64) << 34)
        }
        6 => {
            (first >> 7) as u64
                | ((bytes[*offset - 6] as u64) << 1)
                | ((bytes[*offset - 5] as u64) << 9)
                | ((bytes[*offset - 4] as u64) << 17)
                | ((bytes[*offset - 3] as u64) << 25)
                | ((bytes[*offset - 2] as u64) << 33)
                | ((bytes[*offset - 1] as u64) << 41)
        }
        7 => {
            (bytes[*offset - 7] as u64)
                | ((bytes[*offset - 6] as u64) << 8)
                | ((bytes[*offset - 5] as u64) << 16)
                | ((bytes[*offset - 4] as u64) << 24)
                | ((bytes[*offset - 3] as u64) << 32)
                | ((bytes[*offset - 2] as u64) << 40)
                | ((bytes[*offset - 1] as u64) << 48)
        }
        8 => {
            (bytes[*offset - 8] as u64)
                | ((bytes[*offset - 7] as u64) << 8)
                | ((bytes[*offset - 6] as u64) << 16)
                | ((bytes[*offset - 5] as u64) << 24)
                | ((bytes[*offset - 4] as u64) << 32)
                | ((bytes[*offset - 3] as u64) << 40)
                | ((bytes[*offset - 2] as u64) << 48)
                | ((bytes[*offset - 1] as u64) << 56)
        }
        _ => unreachable!(),
    };
    *offset = offset.wrapping_sub((shift + 1) as usize);
    Ok(result)
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "encode", feature = "decode"))]
    use super::super::tests::round_trip;
    use super::*;

    #[cfg(all(feature = "encode", feature = "decode"))]
    fn round_trip_prefix(values: &[u64]) -> DecodeResult<()> {
        round_trip(values, encode_prefix_varint, decode_prefix_varint)
    }

    #[cfg(all(feature = "encode", feature = "decode"))]
    fn round_trip_suffix(values: &[u64]) -> DecodeResult<()> {
        let mut bytes = Vec::new();
        for value in values.iter() {
            encode_suffix_varint(*value, &mut bytes);
        }

        let mut result = Vec::new();
        let mut offset = bytes.len().wrapping_sub(1);
        while offset != std::usize::MAX {
            let next = decode_suffix_varint(&bytes, &mut offset)?;
            result.push(next);
        }
        result.reverse();

        assert_eq!(&result, &values);
        Ok(())
    }

    #[cfg(all(feature = "encode", feature = "decode"))]
    #[test]
    fn test_prefix() -> DecodeResult<()> {
        let vecs = vec![vec![99, 127, 128, 0, 1, 2, 3, std::u64::MAX]];
        for vec in vecs.iter() {
            round_trip_prefix(vec)?;
        }

        // All the numbers with between 0 and 3 bits set
        let mut vec = Vec::new();

        for a in 0..64 {
            for b in 0..64 {
                for c in 0..64 {
                    let num = (1u64 << a) | (1u64 << b) | (1u64 << c);
                    vec.push(num);
                }
                round_trip_prefix(&vec)?;
                vec.clear();
            }
        }
        Ok(())
    }

    #[cfg(all(feature = "encode", feature = "decode"))]
    #[test]
    fn test_suffix() -> DecodeResult<()> {
        let vecs = vec![vec![99, 127, 128, 0, 1, 2, 3, std::u64::MAX]];
        for vec in vecs.iter() {
            round_trip_suffix(vec)?;
        }

        // All the numbers with between 0 and 2 bits set
        // (up to 3 bits was tested, as can be seen below)
        let mut vec = Vec::new();

        for a in 0..64 {
            for b in 0..64 {
                for c in 0..64 {
                    let num = (1u64 << a) | (1u64 << b) | (1u64 << c);
                    vec.push(num);
                }
                round_trip_suffix(&vec)?;
                vec.clear();
            }
        }

        Ok(())
    }
}
