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
        #[cfg(feature = "write")]
        impl <'a, $($ts: Writable<'a>),+> Writable<'a> for ($($ts),+) {
            type WriterArray=($($ts::WriterArray),+);
            fn write_root<'b: 'a>(&'b self, stream: &mut impl WriterStream) -> RootTypeId {
                $(
                    stream.write_with_id(|stream| tuple_index!(self, $ti).write_root(stream));
                )+
                $trid
            }
        }

        #[cfg(feature = "write")]
        impl<'a, $($ts: WriterArray<'a>),+> WriterArray<'a> for ($($ts),+) {
            type Write=($($ts::Write),+);
            fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
                $(
                    tuple_index!(self, $ti).buffer(&tuple_index!(value, $ti));
                )+
            }
            fn flush(self, stream: &mut impl WriterStream) -> ArrayTypeId {
                let ($($ts,)+) = self;
                $(
                    stream.write_with_id(|stream|
                        $ts.flush(stream)
                    );
                )+
                $taid
            }
        }

        #[cfg(feature = "read")]
        impl <$($ts: Readable),+> Readable for ($($ts),+) {
            type ReaderArray=($($ts::ReaderArray),+);
            fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
                match sticks {
                    DynRootBranch::Tuple { mut fields } => {
                        // See also abb368f2-6c99-4c44-8f9f-4b00868adaaf
                        if fields.len() != $count {
                            return Err(ReadError::SchemaMismatch)
                        }
                        let mut fields = fields.drain(..);
                        Ok((
                            // This unwrap is ok because we verified the len already. See alsoa abb368f2-6c99-4c44-8f9f-4b00868adaaf
                            $($ts::read(fields.next().unwrap())?),+
                        ))
                    },
                    _ => Err(ReadError::SchemaMismatch),
                }
            }
        }

        #[cfg(feature = "read")]
        impl <$($ts: ReaderArray),+> ReaderArray for ($($ts),+) {
            type Read=($($ts::Read),+);
            fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
                match sticks {
                    DynArrayBranch::Tuple { mut fields } => {
                        // See also abb368f2-6c99-4c44-8f9f-4b00868adaaf
                        if fields.len() != $count {
                            return Err(ReadError::SchemaMismatch)
                        }
                        let mut fields = fields.drain(..);
                        Ok((
                            // This unwrap is ok because we verified the len already. See alsoa abb368f2-6c99-4c44-8f9f-4b00868adaaf
                            $($ts::new(fields.next().unwrap())?),+
                        ))
                    },
                    _ => Err(ReadError::SchemaMismatch)
                }
            }
            fn read_next(&mut self) -> Self::Read {
                ($(
                    tuple_index!(self, $ti).read_next(),
                )+)
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
