pub mod branch;
pub mod primitive;
pub mod missing;
pub mod error;
pub mod reader_writer;
pub mod prelude;
mod play;

use crate::prelude::*;

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

    let branch = BranchId { name: "", parent: 0 };
    let mut reader = T::Reader::new(&sticks, &branch);
    
    Ok(reader.read())
}

pub fn test_play() {
    play::test();
}


// TODO: When deriving, use the assert implements check that eg: Clone does, to give good compiler errors
//       If this is not possible because it's an internal API, use static_assert


