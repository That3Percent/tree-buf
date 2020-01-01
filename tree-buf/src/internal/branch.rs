use crate::prelude::*;
use crate::internal::encodings::varint::{decode_prefix_varint, encode_prefix_varint};
use std::fmt::Debug;
use std::convert::TryInto;

#[derive(Debug)]
pub struct BranchId<'a> {
    pub name: &'a str,
    // The parent is just the start byte of the parent branch.
    // Every branch must at least write it's primitive id, so these are guaranteed to be unique.
    pub parent: usize,
}

impl<'a> BranchId<'a> {
    pub(crate) fn flush(&self, bytes: &mut Vec<u8>) {
        // Parent, Name length, name bytes
        encode_prefix_varint(self.parent as u64, bytes);
        encode_prefix_varint(self.name.len() as u64, bytes);
        bytes.extend_from_slice(self.name.as_bytes());
    }

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
        let start = *offset;
        // TODO: Store delta instead of end, because it will be smaller.
        // TODO: All the branch data could be flushed at the end of the file using
        // a similar buffering scheme.
        *offset += 8;
        let end = &bytes[start..*offset];
        let end: [u8; 8] = end.try_into().unwrap();
        let end = u64::from_le_bytes(end) as usize;
        let branch = BranchId::read(bytes, offset);
        let BranchId { name, parent } = branch;
        let primitive = bytes[*offset]; // TODO: Prefix varint
        *offset += 1;
        let primitive = PrimitiveId::from_u32(primitive as u32);

        let bytes = &bytes[*offset..end];
        *offset = end;
        Self {
            name,
            parent,
            bytes,
            primitive,
            start,
        }
    }
}
