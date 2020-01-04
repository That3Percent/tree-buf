use crate::prelude::*;
use crate::internal::encodings::varint::{decode_prefix_varint, decode_suffix_varint};
use std::fmt::Debug;

pub mod dyn_branch;
pub mod static_branch;
pub use dyn_branch::*;
pub use static_branch::*;


pub trait StaticBranch : 'static {
    fn children_in_array_context() -> bool;
    fn self_in_array_context() -> bool;
    #[inline(always)]
    fn name(&self) -> Option<&str> { None }
}

#[derive(Debug)]
pub struct BranchId<'a> {
    pub name: &'a str,
    // The parent is just the start byte of the parent branch.
    // Every branch must at least write it's primitive id, so these are guaranteed to be unique.
    pub parent: usize,
}

impl<'a> BranchId<'a> {
    pub(crate) fn read(bytes: &'a [u8], offset: &mut usize) -> Self {
        let parent = decode_prefix_varint(bytes, offset) as usize;
        let str_len = decode_prefix_varint(bytes, offset) as usize;
        let end = *offset + str_len;
        let str_bytes = &bytes[*offset..end];
        *offset = end;
        let name = std::str::from_utf8(str_bytes).unwrap(); // TODO: Error handling
        BranchId { name, parent }
    }

    pub fn find_stick<'s>(&self, sticks: &'s Vec<Stick<'s>>) -> Option<&'s Stick<'s>> {
        sticks.iter().find(|s| s.name == self.name && s.parent == self.parent)
    }
}

// TODO: Make whether the stick has a name contextual based on the type of the parent
// to save space in the file. Eg: Option & Array children need no name
#[derive(Debug)]
pub struct Stick<'a> {
    pub(crate) name: &'a str,
    pub(crate) parent: usize,
    pub(crate) bytes: &'a [u8],
    pub(crate) primitive: PrimitiveId,
    pub start: usize,
}

impl<'a> Stick<'a> {
    pub(crate) fn read(bytes: &'a [u8], offset: &mut usize) -> Self {
        // See also {2d1e8f90-c77d-488c-a41f-ce0fe3368712}
        let len = decode_suffix_varint(bytes, offset) as usize;
        let end = *offset + 1;
        let start = end - len;
        *offset = start - 1;

        // Re-let offset for our own use, since we want to preserve
        // the beginning
        let mut offset = start;
        // TODO: All the branch data could be flushed at the end of the file using
        // a similar buffering scheme.
        let branch = BranchId::read(bytes, &mut offset);
        let primitive = bytes[offset]; // TODO: Prefix varint
        offset += 1;
        let primitive = PrimitiveId::from_u32(primitive as u32);

        Self {
            name: branch.name,
            parent: branch.parent,
            bytes: &bytes[offset..end],
            primitive,
            start,
        }
    }
}
