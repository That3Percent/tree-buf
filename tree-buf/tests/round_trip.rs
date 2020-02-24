use std::fmt::Debug;
#[cfg(not(debug_assertions))]
use std::time::{Duration, Instant};
use tree_buf::prelude::*;
mod common;
use common::*;

// Create this namespace to hide the prelude. This is a check that the hygenics do not require any types from tree_buf to be imported
mod hide_namespace {
    use tree_buf::{Read, Write};
    #[derive(Read, Write, PartialEq, Debug, Clone)]
    pub struct Bits {
        pub f: f64,
        pub obj_array: Vec<Bobs>,
        pub extra: Option<Bobs>,
        pub s: Box<String>,
    }

    #[derive(Read, Write, PartialEq, Debug, Clone)]
    pub struct Bobs {
        pub one: Vec<u64>,
        pub tup: (f64, f64),
    }
}

use hide_namespace::{Bits, Bobs};

// TODO: Compare to Avro - https://github.com/flavray/avro-rs

fn make_item() -> Bits {
    Bits {
        f: 5.0,
        extra: Some(Bobs {
            one: vec![99],
            tup: (9999.99, 200.1),
        }),
        s: Box::new("abc".to_owned()),
        obj_array: vec![
            Bobs {
                one: vec![3, 2, 1, 0],
                tup: (10.0, 200.2),
            },
            Bobs { one: vec![], tup: (2.2, 200.3) },
            Bobs {
                one: vec![20, 20, 20, 20, 20, 20, 20],
                tup: (0.0, 200.4),
            },
        ],
    }
}

#[test]
fn bools_root() {
    round_trip(&true, 1);
    round_trip(&false, 1);
}

#[test]
fn opts_root() {
    round_trip(&Some(true), 1);
    round_trip(&Option::<bool>::None, 1);
}

#[test]
fn ints_root() {
    round_trip(&0u32, 1);
    round_trip(&1u32, 1);
    for i in 2..=255u32 {
        round_trip(&i, 2);
    }
    for i in 256..1024u32 {
        round_trip(&i, 3);
    }
}

// Special case for 1 element array encodes root object
#[test]
fn array1() {
    round_trip(&vec![99u64], 3);
    round_trip(&vec![1u64], 2);
}

#[test]
fn int_vec() {
    round_trip(&vec![99u64, 100], 6);
}

#[test]
fn float_vec() {
    round_trip(&vec![0.99], 10);
}

#[test]
fn nested_float_vec() {
    round_trip(&vec![vec![10.0, 11.0], vec![], vec![99.0]], 34);
}

#[test]
fn array_tuple() {
    round_trip(&vec![vec![(1u32, 2u32), (3, 4), (5, 6)]], 14);
}

#[test]
fn round_trip_item() {
    let item = make_item();
    round_trip(&item, 144);
}

#[test]
fn round_trip_item_vec() {
    let item = make_item();
    let item = vec![item; 5];
    round_trip(&item, 511);
}

#[test]
fn nullable_array() {
    round_trip(&vec![Some(1u32), Some(2)], 9);
}

#[test]
fn visibility_modifiers() {
    #[derive(Default, Read, Write, Debug, PartialEq)]
    struct Inherited {
        a: u64,
    }

    #[derive(Default, Read, Write, Debug, PartialEq)]
    pub(crate) struct Crate {
        a: u64,
    }

    #[derive(Default, Read, Write, Debug, PartialEq)]
    pub struct Public {
        a: u64,
    }

    round_trip_default::<Inherited>(4);
    round_trip_default::<Crate>(4);
    round_trip_default::<Public>(4);
}

// TODO: Using Quickcheck and Arbitrary with quickcheck_derive.
#[test]
fn various_types() {
    // TODO: u32 - u8

    round_trip_default::<u64>(1);
    //round_trip_default::<u32>(1);
    //round_trip_default::<u16>(1);
    //round_trip_default::<u8>(1);
    round_trip_default::<(u64, u64)>(3);
    //round_trip_default::<(u64, u32)>(3);
    round_trip_default::<f64>(1);
    //round_trip_default::<Vec<u32>>(3);
    //round_trip_default::<Option<Vec<u32>>>(3);
    // TODO: u32
    //round_trip_default::<Option<u32>>(2);
    // TODO: u32
    //round_trip_default::<Vec<Option<u32>>>(2);
    round_trip_default::<String>(1);
}

#[test]
fn conversions() {
    // TODO: f32
    //serialize_eq(1.0f64, 1.0f32, 0);
    //serialize_eq(1.0f32, 1.0f64, 0);
    //serialize_eq(9.0f32, 9.0f64, 0);

    // TODO: A bunch more of these
}

#[test]
fn small_structs() {
    #[derive(Read, Write, Default, Debug, PartialEq)]
    struct _1 {
        a: u64,
    }

    round_trip_default::<_1>(4);
}

#[test]
fn large_structs() {
    #[derive(Read, Write, Default, Debug, PartialEq)]
    struct _14 {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
        g: f64,
        h: f64,
        i: f64,
        j: f64,
        k: f64,
        l: f64,
        m: f64,
        n: f64,
    }

    #[derive(Read, Write, Default, Debug, PartialEq)]
    struct _15 {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
        g: f64,
        h: f64,
        i: f64,
        j: f64,
        k: f64,
        l: f64,
        m: f64,
        n: f64,
        o: f64,
    }

    #[derive(Read, Write, Default, Debug, PartialEq)]
    struct _16 {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
        g: f64,
        h: f64,
        i: f64,
        j: f64,
        k: f64,
        l: f64,
        m: f64,
        n: f64,
        o: f64,
        p: f64,
    }
    // TODO: Match privacy in derive macro from struct deriving to writer impls
    #[derive(Read, Write, Default, Debug, PartialEq)]
    struct _17 {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
        g: f64,
        h: f64,
        i: f64,
        j: f64,
        k: f64,
        l: f64,
        m: f64,
        n: f64,
        o: f64,
        p: f64,
        q: f64,
    }

    round_trip_default::<_14>(44);
    round_trip_default::<_15>(47);
    round_trip_default::<_16>(50);
    round_trip_default::<_17>(53);
}
