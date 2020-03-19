mod common;
use common::round_trip;
use tree_buf::prelude::*;
#[test]
fn root_1_unnamed() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum K {
        String(String),
    }

    round_trip(&K::String("s".to_owned()), 0);
}

#[test]
fn array_1_unnamed() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum K {
        String(String),
    }

    round_trip(&vec![K::String("s".to_owned()), K::String("k".to_owned())], 0);
}
/*
#[test]
fn visibility_modifiers() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum Priv {
        Val(u32),
    }

    round_trip(&Priv::Val(10), 0);

    #[derive(Read, Write, Debug, PartialEq)]
    pub enum Pub {
        Val(u32),
    }

    round_trip(&Pub::Val(10), 0);
}
*/
/*

#[test]
fn selects_correct_discriminant() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum Opts {
        One(u32),
        Two(u8),
    }

    round_trip(&Opts::One(1), 0);
    round_trip(&Opts::Two(2), 0);
}

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
