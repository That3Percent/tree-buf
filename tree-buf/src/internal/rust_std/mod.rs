mod smart_pointers;
mod usize;

pub use self::usize::*;
pub use smart_pointers::*;

/*
impl<'a, T:Writable<'a>> Writable<'a> for &'a T {

}

pub struct RefArrayWriter<T> {

}
*/