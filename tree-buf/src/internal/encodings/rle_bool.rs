use crate::encodings::varint::encode_prefix_varint;
use crate::prelude::*;
use std::vec::IntoIter;

#[cfg(feature = "read")]
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

#[cfg(feature = "write")]
pub fn encode_rle_bool(items: &[bool], bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> Result<ArrayTypeId, ()> {
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

    // TODO: We would probably prefer Simple16 here, but because of a subtle
    // issue we have to use PrefixVar instead. The problem is that compress removes
    // default values from the end of the array, but with Simple16 we don't know
    // for sure if a 0 at the end is a default value or just "padding" from the
    // encoder. This comes at a significant loss to compression in many cases,
    // since it is very likely that most values are small. This would be a problem
    // if we tried to use the compress fn here too.
    // See also 490cf083-7fba-49ea-a14a-41c4ba91a656
    bytes.push(ArrayTypeId::IntPrefixVar.into());
    let len = bytes.len();
    for run in runs {
        encode_prefix_varint(run, bytes);
    }
    lens.push(bytes.len() - len);

    Ok(type_id)
}
