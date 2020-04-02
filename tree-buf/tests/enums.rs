mod common;
use common::round_trip;
use tree_buf::prelude::*;

// TODO: Get a code coverage checker

#[test]
fn unnamed_field_one_variant() {
    #[derive(Read, Write, Debug, PartialEq, Clone)]
    enum K {
        St(String),
    }

    round_trip(&K::St("s".to_owned()), 6, 17);
}

#[test]
fn selects_correct_discriminant() {
    #[derive(Read, Write, Debug, PartialEq, Clone)]
    enum Opts {
        One(u32),
        Two(u8),
    }

    round_trip(&Opts::One(1), 6, 16);
    round_trip(&Opts::Two(2), 7, 16);
}

#[test]
fn pub_vis() {
    #[derive(Read, Write, Debug, PartialEq, Clone)]
    pub enum Pub {
        Val(u32),
    }

    round_trip(&Pub::Val(10), 7, 16);
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

    round_trip(&A::One(1), 6, 16);
    round_trip(&B::One(1), 6, 16);
}

// TODO: Other variations.
/*

#[test]
fn void_value() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum HasVoid {
        One,
        Two
    }

    round_trip(&HasVoid::One, 0, 0);
}

#[test]
fn struct_value() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum HasStruct {
        S { one: u32, two: u32 },
    }

    round_trip(&HasStruct::S { one: 15, two: 15 }, 0, 0);
}
*/
