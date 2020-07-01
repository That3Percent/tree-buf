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

// TODO: Performance: Runs has to be u64 for correctness, but most data won't actually have more than u32 worth (probably even less).
// Consider specializing the impl for different lengths of the items array.
// See also 2a3a69eb-eba1-4c95-9399-f1b9daf48733
#[inline(always)]
fn bool_runs_and_id(items: &[bool]) -> Result<(Vec<u64>, ArrayTypeId), ()> {
    // This encoding is not useful for less than 25 items.
    // 24 items requires 3 bytes for PackedBool, and that is the
    // minimum possible for this encoding to just identify the runs
    // type id, the len of the runs bytes buffer, and at least 1 value
    // encoded in there as some non-zero int.
    if items.len() < 25 {
        return Err(());
    }

    let mut current_value = items[0];
    let type_id = if current_value { ArrayTypeId::RLEBoolTrue } else { ArrayTypeId::RLEBoolFalse };
    let mut current_run: u64 = 0;
    // TODO: (Performance) use second-stack
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

    Ok((runs, type_id))
}

#[cfg(feature = "encode")]
pub fn encode_rle_bool<O: EncodeOptions>(items: &[bool], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
    profile_fn!(encode_rle_bool);

    let (runs, type_id) = bool_runs_and_id(items)?;
    stream.encode_with_id(|stream| runs.flush(stream));

    Ok(type_id)
}

#[cfg(feature = "encode")]
pub fn size_of_rle_bool<O: EncodeOptions>(items: &[bool], options: &O) -> Result<usize, ()> {
    let (runs, _) = bool_runs_and_id(items)?;
    let runs_size = Vec::<u64>::fast_size_for_all(&runs[..], options);
    // + the type id for runs
    Ok(runs_size + 1)
}
