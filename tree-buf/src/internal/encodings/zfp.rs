use super::varint::{decode_prefix_varint, encode_prefix_varint};
use crate::prelude::*;
use defer::defer;
use std::convert::TryInto;
use std::ffi::c_void;
use std::ptr::null_mut;
use zfp_sys_cc::*;
use zigzag::ZigZag;

pub(crate) trait ZfpKind: Default + Copy {
    fn data_type() -> zfp_type;
    fn array_type_id() -> ArrayTypeId;
}

impl ZfpKind for f64 {
    fn data_type() -> zfp_type {
        zfp_type_zfp_type_double
    }
    fn array_type_id() -> ArrayTypeId {
        ArrayTypeId::Zfp64
    }
}

impl ZfpKind for f32 {
    fn data_type() -> zfp_type {
        zfp_type_zfp_type_float
    }
    fn array_type_id() -> ArrayTypeId {
        ArrayTypeId::Zfp32
    }
}

fn field_1d<T: ZfpKind>(data: &[T]) -> Result<(*mut zfp_field, impl Drop), ()> {
    unsafe {
        let data_type = T::data_type();
        let len = data.len().try_into().map_err(|_| ())?;
        let ptr = zfp_field_1d(data.as_ptr() as *mut c_void, data_type, len);
        let guard = defer(move || zfp_field_free(ptr));
        Ok((ptr, guard))
    }
}

fn open_zfp_stream() -> (*mut zfp_stream, impl Drop) {
    unsafe {
        let zfp = zfp_stream_open(null_mut() as *mut bitstream);
        let guard = defer(move || zfp_stream_close(zfp));
        (zfp, guard)
    }
}

fn open_stream(buffer: &[u8], zfp: *mut zfp_stream) -> (*mut bitstream, impl Drop) {
    unsafe {
        let ptr = buffer.as_ptr();
        let stream = stream_open(ptr as *mut c_void, buffer.len() as u64);
        let guard = defer(move || stream_close(stream));
        zfp_stream_set_bit_stream(zfp, stream);
        zfp_stream_rewind(zfp);
        (stream, guard)
    }
}

fn set_accuracy(zfp: *mut zfp_stream, tolerance: Option<i32>) {
    unsafe {
        if let Some(scale) = tolerance {
            let tolerance = (2.0f64).powi(scale);
            zfp_stream_set_accuracy(zfp, tolerance);
        }
    }
}

pub(crate) fn decompress<T: ZfpKind>(bytes: &[u8]) -> ReadResult<Vec<T>> {
    unsafe {
        let mut offset = 0;
        let len = decode_prefix_varint(bytes, &mut offset)?;

        let tolerance = decode_prefix_varint(bytes, &mut offset)?;
        // TODO: FIXME: There are some error conditions here, like overflow,
        // that should be treated as an invalid files
        let tolerance = if tolerance == 0 { None } else { Some(<i64 as ZigZag>::decode(tolerance - 1) as i32) };

        let bytes = &bytes[offset..];

        // TODO: (Performance)
        // Use an uninitialized vec
        let mut data = vec![T::default(); len as usize];
        let (field, _field_guard) = field_1d(&data[..]).map_err(|_| ReadError::InvalidFormat)?;

        // Allocate metadata for the compressed stream
        let (zfp, _zfp_guard) = open_zfp_stream();

        set_accuracy(zfp, tolerance);

        let (_stream, _stream_guard) = open_stream(bytes, zfp);

        let ret = zfp_decompress(zfp, field);
        if ret == 0 {
            return Err(ReadError::InvalidFormat);
        }

        Ok(data)
    }
}

pub(crate) fn compress<T: ZfpKind>(data: &[T], bytes: &mut Vec<u8>, tolerance: Option<i32>) -> Result<ArrayTypeId, ()> {
    // Better to use fixed in this case
    if data.len() == 0 {
        return Err(());
    }
    unsafe {
        // Allocate metadata for the array
        let (field, _field_guard) = field_1d(data)?;

        // Allocate metadata for the compressed stream
        let (zfp, _zfp_guard) = open_zfp_stream();

        // Set tolerance
        set_accuracy(zfp, tolerance);

        // Allocate buffer for compressed data
        let bufsize = zfp_stream_maximum_size(zfp, field);
        let mut buffer: Vec<u8> = vec![0; bufsize as usize];

        // Associate bit stream with allocated buffer
        let (_stream, _stream_guard) = open_stream(&mut buffer[..], zfp);

        // Compress array and output compressed stream
        let zfpsize = zfp_compress(zfp, field) as usize;
        if zfpsize == 0 {
            return Err(());
        }

        // Add the length
        encode_prefix_varint(data.len() as u64, bytes);
        // Add the tolerance
        let tolerance = if let Some(tolerance) = tolerance {
            // TODO: FIXME: Deal with overflow
            ZigZag::encode(tolerance) + 1
        } else {
            0u32
        };
        encode_prefix_varint(tolerance as u64, bytes);

        // TODO: (Performance) Rather than copy buffer,
        // just expand the original, pass that appropriate ptr in, and truncate.
        // The fastest version of this uses MaybeUninit or raw manipulation though,
        // rather than zeroing out.
        bytes.extend_from_slice(&buffer[0..zfpsize]);

        // TODO: (Performance)
        // Read the docs and see if there is anything good to know
        // like header bytes that can be stripped.
        // https://zfp.readthedocs.io/en/release0.5.5/

        Ok(T::array_type_id())
    }
}
