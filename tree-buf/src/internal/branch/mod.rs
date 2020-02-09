use crate::prelude::*;

mod root_branch;
pub use root_branch::*;

mod array_branch;
pub use array_branch::*;

// TODO: There are conceptually 4 pieces which are intermingled in this code.
// 1: The actual 'object model' that TreeBuf uses. Eg:
//     Root values, array values,
// 2: Allowable conversions and downcasts inherant to the object model
// 3: The binary representation of the object model
// 4: The conversion of the object model to Rust data types, like Vec, u8, etc.
//
// It would be good to split these, but not at the cost of performance.
// Doing so would go a long way in guiding a port to another language.
//
// A purist architecture would do each step separately...
// 1: Convert Rust types into Object Model
// 2: Write Object model
// - and in reverse for deserialize
//
// Note that the object model may be just defined in terms of eg: Number, where Number is the sum type of F64, u64, and i64 with downcasts.

#[cfg(feature = "read")]
pub fn read_root(bytes: &[u8]) -> ReadResult<DynRootBranch<'_>> {
    if bytes.len() == 0 {
        return Ok(DynRootBranch::Void);
    }
    let mut lens = bytes.len() - 1;
    let mut offset = 0;
    read_next_root(bytes, &mut offset, &mut lens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::{TryFrom, TryInto};
    fn convert_all<T: TryFrom<u8> + Into<u8>>() {
        for i in 0..=255u8 {
            let v: Result<T, _> = i.try_into();
            if let Ok(v) = v {
                debug_assert_eq!(i, v.into());
            }
        }
    }

    #[cfg(feature = "read")]
    #[test]
    fn all_root_type_ids() {
        convert_all::<RootTypeId>();
    }

    #[cfg(feature = "read")]
    #[test]
    fn all_array_type_ids() {
        convert_all::<ArrayTypeId>();
    }
}
