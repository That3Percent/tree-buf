// TODO: A lot of this is defunct. The fixed array lenght idea though should make it into the PrimitiveId,
// where the PrimitiveId can include extra data itself that applies to all members. This can be extended
// also to include more information about other primitives (eg: ranges for ints)

pub enum ArrayLength {
    Variable,
    Fixed(usize),
}

pub struct Array {
    pub length: ArrayLength,
    pub field_type: FieldType,
}

pub struct Field {
    pub name: String,
    pub offset: usize,
    pub field_type: FieldType,
}

pub struct FieldList {
    fields: Vec<Field>,
}

// TODO: General alignment strategy -
// First bytes of the file can indicate alignment requirement
// Start reading the file, then move to an aligned buffer
//   (one way is to insert bytes to the beginning of the Vec<u8> until it is aligned - but be careful this breaks when the vec resizes so the size must be known upfront)
//   https://users.rust-lang.org/t/alignment-of-vec-data/6785/2


// Type Length = 4 byte little endian u32
// File structure:
// Pre-amble
// 1 byte - 2 >> alignment
// Schema:
//    4 byte u32 - num fields
//    Fields:
//          Parent: length, previous field or 0 for root
//          Name - string - 4 byte
//          Type
// 

