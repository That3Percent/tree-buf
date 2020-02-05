#![allow(non_snake_case)]

use crate::prelude::*;

// https://www.reddit.com/r/rust/comments/339yj3/tuple_indexing_in_a_macro/
macro_rules! expr {
    ($x:expr) => {
        $x
    };
} // HACK
macro_rules! tuple_index {
    ($tuple:expr, $idx:tt) => {
        expr!($tuple.$idx)
    };
}

macro_rules! impl_tuple {
    ($count:expr, $trid:expr, $taid:expr, $($ts:ident, $ti:tt,)+) => {
        impl <'a, $($ts: Writable<'a>),+> Writable<'a> for ($($ts),+) {
            type WriterArray=($($ts::WriterArray),+);
            fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> RootTypeId {
                $(
                    let type_index = bytes.len();
                    bytes.push(0);
                    let type_id = $ts::write_root(&tuple_index!(value, $ti), bytes, lens);
                    bytes[type_index] = type_id.into();
                )+
                $trid
            }
        }

        impl<'a, $($ts: WriterArray<'a>),+> WriterArray<'a> for ($($ts),+) {
            type Write=($($ts::Write),+);
            fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
                $(
                    tuple_index!(self, $ti).buffer(&tuple_index!(value, $ti));
                )+
            }
            fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> ArrayTypeId {
                let ($($ts,)+) = self;
                $(
                    let type_index = bytes.len();
                    bytes.push(0);
                    let type_id = $ts.flush(bytes, lens);
                    bytes[type_index] = type_id.into();
                )+
                $taid
            }
        }

        impl <$($ts: Readable),+> Readable for ($($ts),+) {
            type ReaderArray=($($ts::ReaderArray),+);
            fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
                match sticks {
                    DynRootBranch::Tuple { mut children } => {
                        // See also abb368f2-6c99-4c44-8f9f-4b00868adaaf
                        if children.len() != $count {
                            return Err(ReadError::SchemaMismatch)
                        }
                        let mut children = children.drain(..);
                        Ok((
                            // This unwrap is ok because we verified the len already. See alsoa abb368f2-6c99-4c44-8f9f-4b00868adaaf
                            $($ts::read(children.next().unwrap())?),+
                        ))
                    },
                    _ => Err(ReadError::SchemaMismatch),
                }
            }
        }

        impl <$($ts: ReaderArray),+> ReaderArray for ($($ts),+) {
            type Read=($($ts::Read),+);
            fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
                match sticks {
                    DynArrayBranch::Tuple { mut children } => {
                        // See also abb368f2-6c99-4c44-8f9f-4b00868adaaf
                        if children.len() != $count {
                            return Err(ReadError::SchemaMismatch)
                        }
                        let mut children = children.drain(..);
                        Ok((
                            // This unwrap is ok because we verified the len already. See alsoa abb368f2-6c99-4c44-8f9f-4b00868adaaf
                            $($ts::new(children.next().unwrap())?),+
                        ))
                    },
                    _ => Err(ReadError::SchemaMismatch)
                }
            }
            fn read_next(&mut self) -> ReadResult<Self::Read> {
                Ok(($(
                    tuple_index!(self, $ti).read_next()?,
                )+))
            }
        }
    };
}

// TODO: Consider 0 and 1 sized tuples.
// These should probably be no serialization at all,
// and pass-through serialization respectively and just
// not use the tuple construct. The tuple construct isn't invalid
// though, which opens considerations for matching either for a schema
// which may not be trivial - like a recursive descent parser.
impl_tuple!(2, RootTypeId::Tuple2, ArrayTypeId::Tuple2, T0, 0, T1, 1,);
impl_tuple!(3, RootTypeId::Tuple3, ArrayTypeId::Tuple3, T0, 0, T1, 1, T2, 2,);
impl_tuple!(4, RootTypeId::Tuple4, ArrayTypeId::Tuple4, T0, 0, T1, 1, T2, 2, T3, 3,);
impl_tuple!(5, RootTypeId::Tuple5, ArrayTypeId::Tuple5, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4,);
impl_tuple!(6, RootTypeId::Tuple6, ArrayTypeId::Tuple6, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5,);

// TODO: Support tuple structs in the macro
