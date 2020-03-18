use crate::prelude::*;

// TODO: This is not yet used
pub fn write_object_field<T: TypeId, S: WriterStream>(name: Ident<'_>, f: impl FnOnce(&mut S) -> T, stream: &mut S, num_fields_written: &mut usize) {
    let id = stream.restore_if_void(|stream| {
        write_ident(name, stream.bytes());
        f(stream)
    });
    if id != T::void() {
        *num_fields_written += 1;
    }
}

// TODO: This is not yet used
#[inline]
pub fn write_fields<S: WriterStream, T: TypeId>(max_count: usize, stream: &mut S, f: impl FnOnce(&mut S, &mut usize)) -> usize {
    let mut count = 0;
    if max_count > 8 {
        stream.reserve_and_write_with_varint(max_count as u64, move |stream| {
            f(stream, &mut count);
            count as u64
        });
    } else {
        f(stream, &mut count);
    }
    count
}
