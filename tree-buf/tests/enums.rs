mod common;
use common::round_trip;
use tree_buf::prelude::*;

#[test]
fn unnamed_field_one_variant() {
    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    enum K {
        St(String),
    }

    round_trip(&K::St("s".to_owned()), 6, 16);
}

#[test]
fn selects_correct_discriminant() {
    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    enum Opts {
        One(u32),
        Two(u8),
    }

    round_trip(&Opts::One(1), 6, 14);
    round_trip(&Opts::Two(2), 7, 15);
}

#[test]
fn pub_vis() {
    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    pub enum Pub {
        Val(u32),
    }

    round_trip(&Pub::Val(10), 7, 15);
}

#[test]
fn unused_variations_do_not_affect_size() {
    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    enum A {
        One(u32),
    }
    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    enum B {
        One(u32),
        Two(u32),
    }

    round_trip(&A::One(1), 6, 14);
    round_trip(&B::One(1), 6, 14);
}

#[test]
fn void_value() {
    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    enum HasVoid {
        One,
        Two,
    }

    round_trip(&HasVoid::One, 6, 12);
}

#[test]
fn mixed_void_and_single() {
    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    enum Mixed {
        Ex, // TODO: This can't be named None because of the macro
        One(u32),
    }

    round_trip(&Mixed::Ex, 5, 11);
    round_trip(&Mixed::One(10), 7, 15);

    // FIXME: This increased in size with the fast_size_for change
    // See also 279e9860-d1f6-4a6e-a4bc-1a64c47b8370
    round_trip(&vec![Mixed::Ex, Mixed::One(2), Mixed::One(2), Mixed::One(3), Mixed::Ex], 21, 24);
}

/*
// TODO: Enable test
#[test]
fn wierd_unit_variants() {
    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    enum Unnamed {
        One(),
        Two(),
    }

    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    enum Named {
        One{},
        Two{},
    }

}

// TODO: Enable test
#[test]
fn struct_value() {
    #[derive(Encode, Decode, Debug, PartialEq)]
    enum HasStruct {
        S { one: u32, two: u32 },
    }

    round_trip(&HasStruct::S { one: 15, two: 15 }, 0, 0);
}
*/
