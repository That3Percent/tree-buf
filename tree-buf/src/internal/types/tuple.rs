use crate::prelude::*;

// TODO: I don't know how to get this part in the macro because it would require 2 bindings with the same name given the pattern
// that I usually use.
#[inline(always)]
fn write_two<T0: Writer, T1: Writer>(
    writer: &mut (T0, T1),
    value: &(T0::Write, T1::Write)) {
    writer.0.write(&value.0);
    writer.1.write(&value.1);
}

#[inline(always)]
fn write_three<T0: Writer, T1: Writer, T2: Writer>(
    writer: &mut (T0, T1, T2),
    value: &(T0::Write, T1::Write, T2::Write)) {
    writer.0.write(&value.0);
    writer.1.write(&value.1);
    writer.2.write(&value.2);
}

#[inline(always)]
fn write_four<T0: Writer, T1: Writer, T2: Writer, T3: Writer> (
        writer: &mut (T0, T1, T2, T3),
        value: &(T0::Write, T1::Write, T2::Write, T3::Write)) {
    writer.0.write(&value.0);
    writer.1.write(&value.1);
    writer.2.write(&value.2);
    writer.3.write(&value.3);
}

#[inline(always)]
fn write_five<T0: Writer, T1: Writer, T2: Writer, T3: Writer, T4: Writer> (
        writer: &mut (T0, T1, T2, T3, T4),
        value: &(T0::Write, T1::Write, T2::Write, T3::Write, T4::Write)) {
    writer.0.write(&value.0);
    writer.1.write(&value.1);
    writer.2.write(&value.2);
    writer.3.write(&value.3);
    writer.4.write(&value.4);
}

macro_rules! impl_tuple {
    ($count:expr, $write:ident, $($ts:ident),+) => {
        impl <$($ts: Writable),+> Writable for ($($ts),+) {
            type Writer=($($ts::Writer),+);
        }

        impl <$($ts: Readable),+> Readable for ($($ts),+) {
            type Reader=($($ts::Reader),+);
        }

        impl <$($ts: Writer),+> Writer for ($($ts),+) {
            type Write=($($ts::Write),+);
        
            fn write(&mut self, value: &Self::Write) {
                $write(self, value);
            }
            fn flush<ParentBranch: StaticBranch>(self, branch: ParentBranch, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) {
                PrimitiveId::Tuple { num_fields: $count}.write(bytes);
                let ($($ts,)+) = self;
                $($ts.flush(branch, bytes, lens);)+
            }
            fn new() -> Self {
                (
                    $($ts::new()),+
                )
            }
        }

        
        impl <$($ts: Reader),+> Reader for ($($ts),+) {
            type Read=($($ts::Read),+);
            fn new<ParentBranch: StaticBranch>(sticks: DynBranch<'_>, branch: ParentBranch) -> Self {
                match sticks {
                    DynBranch::Tuple { mut children } => {
                        if children.len() != $count {
                            todo!("schema mismatch");
                        }
                        let mut children = children.drain(..);
                        (
                            $($ts::new(children.next().unwrap(), branch)),+
                        )
                    },
                    _ => todo!("schema mismatch")
                }
            }
            fn read(&mut self) -> Self::Read {
                let ($($ts),+) = self;

                (
                    $($ts.read()),+
                )
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
impl_tuple!(2, write_two, T0, T1);
impl_tuple!(3, write_three, T0, T1, T2);
impl_tuple!(4, write_four, T0, T1, T2, T3);
impl_tuple!(5, write_five, T0, T1, T2, T3, T4);

// TODO: Support tuple structs in the macro