pub mod branch;
pub mod primitive;
pub mod missing;
pub mod error;
pub mod reader_writer;
mod play;

use crate::reader_writer::*;
use crate::error::*;
use crate::branch::*;
use crate::primitive::*;

// TREEBUF
const PREAMBLE: [u8; 7] = [84, 82, 69, 69, 66, 85, 70];

pub fn write<T: Writable>(value: &T) -> Vec<u8>
    //where T::Writer : std::fmt::Debug,
{
    let mut writer = T::Writer::new();
    writer.write(value);
    let mut bytes = Vec::new();
    // The pre-amble is actually necessary for correctness. Without it,
    // the first branch would get written to the beginning and share
    // the same parent id as the null branch.
    bytes.extend_from_slice(&PREAMBLE);
    //print!("{:?}", writer);

    writer.flush(&BranchId { name: "", parent: 0 }, &mut bytes);
    
    bytes
}

#[derive(Debug)]
struct Stick<'a> {
    name: &'a str,
    parent: usize,
    bytes: &'a[u8],
    primitive: PrimitiveId,
}

impl<'a> Stick<'a> {
    fn read(bytes: &'a [u8], offset: &mut usize) -> Self {
        // See also {2d1e8f90-c77d-488c-a41f-ce0fe3368712}
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
        }
    }
}

pub fn read<T: Readable>(bytes: &[u8]) -> Result<T, Error> {
    
    let mut offset = 0;
    assert_eq!(&PREAMBLE, &bytes[offset..offset+PREAMBLE.len()], "Not valid file"); // TODO: Error handling
    offset += PREAMBLE.len();
    let mut sticks = Vec::new();
    while offset < bytes.len() {
        sticks.push(Stick::read(bytes, &mut offset));
    }

    println!("");
    println!("{:?}", sticks);

    todo!();
}

pub fn test_play() {
    play::test();
}


// TODO: When deriving, use the assert implements check that eg: Clone does, to give good compiler errors
//       If this is not possible because it's an internal API, use static_assert


