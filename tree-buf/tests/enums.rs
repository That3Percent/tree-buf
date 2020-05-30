mod common;
use common::round_trip;
use tree_buf::prelude::*;

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

#[test]
fn void_value() {
    #[derive(Read, Write, Debug, PartialEq, Clone)]
    enum HasVoid {
        One,
        Two,
    }

    round_trip(&HasVoid::One, 6, 13);
}

#[test]
fn mixed_void_and_single() {
    #[derive(Read, Write, Debug, PartialEq, Clone)]
    enum Mixed {
        Ex, // TODO: This can't be named None because of the macro
        One(u32),
    }

    round_trip(&Mixed::Ex, 5, 12);
    round_trip(&Mixed::One(10), 7, 16);

    round_trip(&vec![Mixed::Ex, Mixed::One(2), Mixed::One(2), Mixed::One(3), Mixed::Ex], 23, 26);
}

/*
// TODO: Enable test
#[test]
fn wierd_unit_variants() {
    #[derive(Read, Write, Debug, PartialEq, Clone)]
    enum Unnamed {
        One(),
        Two(),
    }

    #[derive(Read, Write, Debug, PartialEq, Clone)]
    enum Named {
        One{},
        Two{},
    }

}

// TODO: Enable test
#[test]
fn struct_value() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum HasStruct {
        S { one: u32, two: u32 },
    }

    round_trip(&HasStruct::S { one: 15, two: 15 }, 0, 0);
}
*/
