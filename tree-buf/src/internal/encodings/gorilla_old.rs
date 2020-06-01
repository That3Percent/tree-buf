use crate::prelude::*;
use gibbon::{
    vec_stream::VecReader,
    DoubleStreamIterator,
};
use num_traits::AsPrimitive;
use std::convert::TryInto as _;
use std::mem::size_of;

pub fn decompress<T: 'static + Copy>(bytes: &[u8]) -> ReadResult<Vec<T>>
where
    f64: AsPrimitive<T>,
{
    // FIXME: Should do schema mismatch for f32 -> f64
    let num_bits_last_elm = bytes.last().ok_or_else(|| ReadError::InvalidFormat)?;
    let bytes = &bytes[..bytes.len() - 1];
    let last = &bytes[bytes.len() - (bytes.len() % 8)..];
    let bytes = &bytes[..bytes.len() - last.len()];
    let mut last_2 = [0u8; 8];
    for (i, value) in last.iter().enumerate() {
        last_2[i + (8 - last.len())] = *value;
    }
    let last = u64::from_le_bytes(last_2);
    // TODO: Change this to check that num_bits_last_elm is correct
    if bytes.len() % size_of::<u64>() != 0 {
        return Err(ReadError::InvalidFormat);
    }
    // TODO: (Performance) The following can use unchecked, since we just verified the size is valid.
    let mut data = read_all(bytes, |bytes, offset| {
        let start = *offset;
        let end = start + size_of::<u64>();
        let le_bytes = &bytes[start..end];
        *offset = end;
        let result = u64::from_le_bytes(le_bytes.try_into().unwrap());
        Ok(result)
    })?;
    data.push(last);
    #[cfg(feature = "profile")]
    flame::start("Construct");
    let reader = VecReader::new(&data, *num_bits_last_elm);
    let iterator = DoubleStreamIterator::new(reader);
    #[cfg(feature = "profile")]
    flame::end("Construct");
    // FIXME: It seems like this collect can panic if the data is invalid.
    #[cfg(feature = "profile")]
    flame::start("Collect");
    let values: Vec<_> = iterator.map(|v| v.as_()).collect();
    #[cfg(feature = "profile")]
    flame::end("Collect");
    Ok(values)
}
