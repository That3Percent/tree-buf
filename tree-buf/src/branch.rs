use std::fmt::Debug;
use crate::prelude::*;

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
        self.parent.write(bytes);
        self.name.len().write(bytes);
        bytes.extend_from_slice(self.name.as_bytes());
    }

    pub(crate) fn read(bytes: &'a [u8], offset: &mut usize) -> Self {
        let parent: usize = EzBytes::read_bytes(bytes, offset);
        let str_len: usize = EzBytes::read_bytes(bytes, offset);
        let end = *offset+str_len;
        let str_bytes = &bytes[*offset..end];
        *offset = end;
        let name = std::str::from_utf8(str_bytes).unwrap(); // TODO: Error handling
        BranchId {
            name,
            parent,
        }
    }

    pub fn find_stick<'s>(&self, sticks: &'s Vec<Stick<'s>>) -> Option<&'s Stick<'s>> {
        sticks.iter().find(|s| s.name == self.name && s.parent == self.parent)
    }
}

#[derive(Debug)]
pub struct Stick<'a> {
    pub(crate) name: &'a str,
    pub(crate) parent: usize,
    pub(crate) bytes: &'a[u8],
    pub(crate) primitive: PrimitiveId,
    pub(crate) start: usize,
}

impl<'a> Stick<'a> {
    pub(crate) fn read(bytes: &'a [u8], offset: &mut usize) -> Self {
        // See also {2d1e8f90-c77d-488c-a41f-ce0fe3368712}
        let start = *offset;
        let end = usize::read_bytes(bytes, offset);
        let branch = BranchId::read(bytes, offset);
        let BranchId { name, parent } = branch;
        let primitive: u32 = EzBytes::read_bytes(bytes, offset);
        let primitive = PrimitiveId::from_u32(primitive);

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
