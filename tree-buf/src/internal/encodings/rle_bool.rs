use crate::prelude::*;
use std::vec::IntoIter;

#[cfg(feature = "decode")]
pub fn decode_rle_bool(runs: IntoIter<u64>, first: bool) -> IntoIter<bool> {
    let mut results = Vec::new();
    let mut current = first;

    for run in runs {
        for _ in 0..=run {
            results.push(current);
        }
        current = !current;
    }

    results.into_iter()
}

#[cfg(feature = "encode")]
pub fn encode_rle_bool<O: EncodeOptions>(items: &[bool], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
    profile!(&[bool], "encode_rle_bool");

    // This encoding is not useful for this case, since we can store 8 values
    // in 1 byte for PackedBool. Also prevents panic later.
    if items.len() < 8 {
        return Err(());
    }

    let mut current_value = items[0];
    let type_id = if current_value { ArrayTypeId::RLEBoolTrue } else { ArrayTypeId::RLEBoolFalse };
    let mut current_run: u64 = 0;
    let mut runs = Vec::new();
    let items = &items[1..];
    for item in items {
        if *item == current_value {
            current_run += 1;
        } else {
            current_value = *item;
            runs.push(current_run);
            current_run = 0;
        }
    }
    runs.push(current_run);

    stream.encode_with_id(|stream| runs.flush(stream));

    Ok(type_id)
}
