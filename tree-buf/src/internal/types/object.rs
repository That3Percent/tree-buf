use crate::prelude::*;

// TODO: This is not yet used
pub fn encode_object_field<'a, T: TypeId, O: EncodeOptions>(
    name: Ident<'_>,
    f: impl FnOnce(&mut EncoderStream<'a, O>) -> T,
    stream: &mut EncoderStream<'a, O>,
    num_fields_written: &mut usize,
) {
    let id = stream.restore_if_void(|stream| {
        encode_ident(name, stream);
        f(stream)
    });
    if id != T::void() {
        *num_fields_written += 1;
    }
}

// TODO: This is not yet used
#[inline]
pub fn encode_fields<'a, O: EncodeOptions, T: TypeId>(max_count: usize, stream: &mut EncoderStream<'a, O>, f: impl FnOnce(&mut EncoderStream<'a, O>, &mut usize)) -> usize {
    let mut count = 0;
    if max_count > 8 {
        stream.reserve_and_encode_with_varint(max_count as u64, move |stream| {
            f(stream, &mut count);
            count as u64
        });
    } else {
        f(stream, &mut count);
    }
    count
}
