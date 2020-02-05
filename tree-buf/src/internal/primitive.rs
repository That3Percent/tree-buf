use crate::internal::encodings::varint::decode_prefix_varint;
use crate::prelude::*;
use std::vec::IntoIter;

// TODO: The idea for int is to always encode up to 64 bit values,
// but for any data store the min value and offset first, then use
// that to select an optimal encoding. When deserializing, the min and
// offset can be used to find if the data type required by the schema
// matches.
// Consider something like this - https://lemire.me/blog/2012/09/12/fast-integer-compression-decoding-billions-of-integers-per-second/

// TODO: Bytes = [u8]
// TODO: Date
// TODO: Enum - Something like this... needs to simmer.
//              The enum primitive id contains 1 number which is the discriminant count.
//              The enum discriminant as int is contained in the enum branch
//              Each sub-branch contains the discriminant name (string)
//              Each branch may have a sub-branch for data belonging to the variant for that discriminant in each entry.
//              In many cases, this data will be Void, which may be wasteful to have a branch for.
//              ..
//              Because enum is so flexible, it's possible to wrap some dynamic data into it. Eg: EnumValue<T>.
//              This would create some number of sub-branches 'dynamically'.

// Total slots: 256
// TODO: Try each compression on a sample of the data (first 1024 or so?) in turn to decide which to use.
// 1-Reserved for adding features
// 16-Object & Fields
// 16-Tuple & Fields
// 8-Array & different fixed/variable sizes - 0,1,2,128,custom(follows). Fixed 0 necessarily has Void child
// ? Integer - Different for array context or not? Min/Max? Different encoding options? (uncompressed option) signed, unsigned, 8,16,32,64
// ?-Enum - String,Int, or other discriminant, whether or not there is data for sub-branches, and whether
// 1-Nullable
// 1-Boolean
// 4-Float (32/64, compresssed/not) Consider:
//      dfcm - https://userweb.cs.txstate.edu/~mb92/papers/dcc06.pdf
//      https://www.cs.unc.edu/~isenburg/lcpfpv/
//      https://akumuli.org/akumuli/2017/02/05/compression_part2/
//      Consider an 'allow-lossy' flag (per field) or input trait
// 1-Void
// 2-String - compressed, uncompressed
// 1-128 bits
// 2-Blob - compressed, uncompressed
// 1-magic number (preamble)

// TODO: Come back to usize
// TODO: Error check that the result fits in the platform size
pub fn read_usize(bytes: &[u8], offset: &mut usize) -> ReadResult<usize> {
    Ok(decode_prefix_varint(bytes, offset)? as usize)
}

/*

impl Readable for usize {
    type ReaderArray = IntoIter<usize>;
    fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
        Ok(u64::read(sticks)? as Self)
    }
}

*/

impl ReaderArray for IntoIter<usize> {
    type Read = usize;
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
        todo!();
    }
    fn read_next(&mut self) -> ReadResult<Self::Read> {
        self.next().ok_or_else(|| ReadError::InvalidFormat(InvalidFormat::ShortArray))
    }
}
