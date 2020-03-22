mod common;
use common::round_trip;
use tree_buf::prelude::*;
use tree_buf::{Readable, Writable};

// TODO: Get a code coverage checker

// TODO: Move all round trip tests to cover both the array case and the root case.
fn round_trips<'a, 'b: 'a, T: Writable<'a> + Readable + Clone + std::fmt::Debug + PartialEq + 'static>(value: &'b T, root_size: usize, array_size: usize) {
    round_trip(value, root_size);
    let v = vec![value.clone(), value.clone()];
    // Hack! What's up with the borrow checker here?
    let slice: &'static Vec<T> = unsafe { std::mem::transmute(&v) };
    round_trip(slice, array_size);
}

#[test]
fn unnamed_field_one_variant() {
    #[derive(Read, Write, Debug, PartialEq, Clone)]
    enum K {
        St(String),
    }

    round_trips(&K::St("s".to_owned()), 6, 17);
}

#[test]
fn selects_correct_discriminant() {
    #[derive(Read, Write, Debug, PartialEq, Clone)]
    enum Opts {
        One(u32),
        Two(u8),
    }

    round_trips(&Opts::One(1), 6, 16);
    round_trips(&Opts::Two(2), 7, 16);
}

#[test]
fn pub_vis() {
    #[derive(Read, Write, Debug, PartialEq, Clone)]
    pub enum Pub {
        Val(u32),
    }

    round_trips(&Pub::Val(10), 7, 16);
}

#[test]
fn unused_variations_do_not_affect_size() {
    #[derive(Read, Write, Debug, PartialEq, Clone)]
    enum A {
        One(u32),
    }
    #[derive(Read, Write, Debug, PartialEq, Clone)]
    enum B {
        One(u32),
        Two(u32),
    }

    round_trips(&A::One(1), 6, 16);
    round_trips(&B::One(1), 6, 16);
}

/*

#[test]
fn void_value() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum HasVoid {
        One,
        Two
    }

    round_trip(&HasVoid::One, 0);
}

#[test]
fn struct_value() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum HasStruct {
        S { one: u32, two: u32 },
    }

    round_trip(&HasStruct::S { one: 15, two: 15 }, 0);
}
*/
