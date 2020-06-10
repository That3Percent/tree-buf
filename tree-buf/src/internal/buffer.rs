use std::alloc::{alloc, dealloc, Layout};
use std::mem::{align_of, size_of, transmute};
use std::ops::Deref;
use std::ptr::{self, NonNull};
use std::slice;

const SIZE: usize = 2 * 64 * 1024;
const ALIGN: usize = 64;
const LAYOUT: Layout = unsafe { Layout::from_size_align_unchecked(SIZE, ALIGN) };

/// This is an unsafe implementation of Vec with a fixed capacity.
/// It does not support: zsts, drop, or items with alignment > 64.
/// It is always allocated at full capacity when constructed, and can
/// be re-used for other types also satisfying the constraints
pub(crate) struct Buffer<T> {
    ptr: NonNull<T>,
    len: usize,
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        unsafe { dealloc(self.base() as *mut u8, LAYOUT) }
    }
}

// Ensures that the type can be stored in buffer
#[inline(always)]
fn check_buffer<T: Copy>() {
    // Not a ZST
    assert!(size_of::<T>() > 0);
    // Fits evenly inside the buffer
    assert!(SIZE % size_of::<T>() == 0);
    // Alignment requirement met
    assert!(align_of::<T>() <= ALIGN);
}

impl<T: Copy> Default for Buffer<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy> Buffer<T> {
    pub fn new() -> Self {
        check_buffer::<T>();
        unsafe {
            let ptr = alloc(LAYOUT);
            let ptr = transmute(ptr);
            let ptr = NonNull::new(ptr).expect("Failed to allocate buffer");
            Self { ptr, len: 0 }
        }
    }
}

impl<T> Deref for Buffer<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.base(), self.len) }
    }
}

impl<T> Buffer<T> {
    pub const CAPACITY: usize = SIZE / size_of::<T>();

    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        Self::CAPACITY
    }

    /// Panics: If len != 0
    pub fn transmute_empty<O: Copy>(self) -> Buffer<O> {
        check_buffer::<O>();
        assert!(self.len == 0);
        unsafe { Buffer { len: 0, ptr: transmute(self.ptr) } }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn try_push(&mut self, elem: T) -> Result<(), T> {
        if self.len == self.capacity() {
            return Err(elem);
        }
        unsafe { ptr::write(self.top(), elem) }
        self.len += 1;
        Ok(())
    }

    // TODO: Method for take n from start

    /// Extends the slice as much as possible, returning any remainder
    pub fn try_extend<'a>(&'_ mut self, slice: &'a [T]) -> Result<(), &'a [T]> {
        let take = slice.len().min(self.capacity() - self.len);
        unsafe {
            ptr::copy_nonoverlapping(slice.as_ptr(), self.top(), take);
        }
        self.len += take;
        if take != slice.len() {
            Err(&slice[take..])
        } else {
            Ok(())
        }
    }

    #[inline]
    fn top(&self) -> *mut T {
        unsafe { self.base().offset(self.len as isize) }
    }

    #[inline]
    fn base(&self) -> *mut T {
        self.ptr.as_ptr()
    }
}

#[derive(Default)]
pub(crate) struct BufferPool {
    pool: Vec<Buffer<u8>>,
}

impl BufferPool {
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns an empty Buffer<T>.
    pub fn take<T: Copy>(&mut self) -> Buffer<T> {
        self.pool.pop().unwrap_or_default().transmute_empty()
    }
    /// Puts a Buffer<T> into the pool for later.
    pub fn put<T>(&mut self, buffer: Buffer<T>) {
        self.pool.push(buffer.transmute_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    pub fn can_push_to_capacity() {
        let mut buffer = Buffer::new();
        for i in 0..buffer.capacity() {
            assert_eq!(buffer.len(), i);
            assert_eq!(Ok(()), buffer.try_push(i));
        }
        assert_eq!(buffer.len(), buffer.capacity());
        assert_eq!(Err(1), buffer.try_push(1));
        assert_eq!(Err(100), buffer.try_push(100));
    }

    #[test]
    pub fn can_extend() {
        // Try just a normal extend
        let mut buffer = Buffer::new();
        let mut data = vec![100u32, 20, 20, 10, 0];
        assert_eq!(Ok(()), buffer.try_extend(&data[..]));
        assert_eq!(&data[..], &buffer[..]);

        // And another
        let extension = vec![10, 9, 8, 7, 6, 5, 4];
        data.extend_from_slice(&extension[..]);
        assert_eq!(Ok(()), buffer.try_extend(&extension[..]));
        assert_eq!(&data[..], &buffer[..]);

        // Try extending beyond the capacity
        let long: Vec<_> = (0..buffer.capacity() as u32).collect();
        data.extend_from_slice(&long[..]);
        let err = buffer.try_extend(&long[..]);
        let too_much = &data[buffer.capacity()..];
        assert_eq!(Err(too_much), err);
        assert_eq!(&data[0..buffer.capacity()], &buffer[..]);
    }

    #[test]
    pub fn deref() {
        let mut buffer = Buffer::new();

        let data = vec![0u8, 1, 255, 12];
        for elem in data.iter() {
            buffer.try_push(*elem).unwrap();
        }

        assert_eq!(&data[..], &buffer[..]);
    }
}
