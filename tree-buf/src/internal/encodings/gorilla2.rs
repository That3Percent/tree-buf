use crate::prelude::*;
use num_traits::AsPrimitive;
use std::convert::TryInto as _;
use std::mem::size_of;

pub fn decompress<T: 'static + Copy>(bytes: &[u8]) -> ReadResult<Vec<T>>
where
    f64: AsPrimitive<T>,
{
    todo!()
}

pub fn compress(data: impl Iterator<Item = f64> + ExactSizeIterator, bytes: &mut Vec<u8>) -> Result<ArrayTypeId, ()> {
    todo!()
}
