#![allow(non_snake_case)]

use crate::prelude::*;


// https://www.reddit.com/r/rust/comments/339yj3/tuple_indexing_in_a_macro/
macro_rules! expr { ($x:expr) => ($x) } // HACK
macro_rules! tuple_index {
    ($tuple:expr, $idx:tt) => { expr!($tuple.$idx) }
}

macro_rules! impl_tuple {
    ($count:expr, $($ts:ident, $ti:tt,)+) => {
        impl <'a, $($ts: Writable<'a>),+> Writable<'a> for ($($ts),+) {
            type Writer=($($ts::Writer),+);
        }

        impl <$($ts: Readable),+> Readable for ($($ts),+) {
            type Reader=($($ts::Reader),+);
        }

        impl <'a, $($ts: Writer<'a>),+> Writer<'a> for ($($ts),+) {
            type Write=($($ts::Write),+);
        
            fn write<'b : 'a>(&mut self, value: &'b Self::Write) {
                $(tuple_index!(self, $ti).write(&tuple_index!(value, $ti));)+
            }
            fn flush<ParentBranch: StaticBranch>(self, branch: ParentBranch, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) {
                PrimitiveId::Tuple { num_fields: $count}.write(bytes);
                let ($($ts,)+) = self;
                $($ts.flush(branch, bytes, lens);)+
            }
        }

        
        impl <$($ts: Reader),+> Reader for ($($ts),+) {
            type Read=($($ts::Read),+);
            fn new<ParentBranch: StaticBranch>(sticks: DynBranch<'_>, branch: ParentBranch) -> ReadResult<Self> {
                match sticks {
                    DynBranch::Tuple { mut children } => {
                        // See also abb368f2-6c99-4c44-8f9f-4b00868adaaf
                        if children.len() != $count {
                            return Err(ReadError::SchemaMismatch)
                        }
                        let mut children = children.drain(..);
                        Ok((
                            // This unwrap is ok because we verified the len already. See alsoa abb368f2-6c99-4c44-8f9f-4b00868adaaf
                            $($ts::new(children.next().unwrap(), branch)?),+
                        ))
                    },
                    _ => Err(ReadError::SchemaMismatch)
                }
            }
            fn read(&mut self) -> ReadResult<Self::Read> {
                Ok((
                    $(tuple_index!(self, $ti).read()?),+
                ))
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
impl_tuple!(2, T0, 0, T1, 1,);
impl_tuple!(3, T0, 0, T1, 1, T2, 2,);
impl_tuple!(4, T0, 0, T1, 1, T2, 2, T3, 3,);
impl_tuple!(5, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4,);
impl_tuple!(6, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5,);

// TODO: Support tuple structs in the macro