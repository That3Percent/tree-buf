use std::fmt::Debug;
#[cfg(not(debug_assertions))]
use std::time::{Duration, Instant};
use tree_buf::prelude::*;
mod common;
use common::*;

// Create this namespace to hide the prelude. This is a check that the hygenics do not require any types from tree_buf to be imported
mod hide_namespace {
    use serde::{Deserialize, Serialize};
    use tree_buf::{Read, Write};
    #[derive(Serialize, Deserialize, Read, Write, PartialEq, Debug, Clone)]
    pub struct Bits {
        pub f: f64,
        pub obj_array: Vec<Bobs>,
        pub extra: Option<Bobs>,
        pub s: Box<String>,
    }

    #[derive(Serialize, Deserialize, Read, Write, PartialEq, Debug, Clone)]
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
    round_trip(&0, 1);
    round_trip(&1, 1);
    for i in 2..=255 {
        round_trip(&i, 2);
    }
    for i in 256..1024 {
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
    round_trip(&vec![vec![10.0, 11.0], vec![], vec![99.0]], 33);
}

#[test]
fn array_tuple() {
    round_trip(&vec![vec![(1, 2), (3, 4), (5, 6)]], 14);
}

#[test]
fn round_trip_item() {
    let item = make_item();
    round_trip(&item, 146);
}

#[test]
fn round_trip_item_vec() {
    let item = make_item();
    let item = vec![item; 5];
    round_trip(&item, 532);
}

#[test]
fn nullable_array() {
    round_trip(&vec![Some(1), Some(2)], 9);
}

#[cfg(not(debug_assertions))]
fn bad_benchmark(f: impl Fn()) -> Duration {
    // Warmup
    for _ in 0..10000 {
        f();
    }

    let start = Instant::now();
    for _ in 0..100000 {
        f();
    }
    let end = Instant::now();
    (end - start) / 100000
}

// TODO: Move these tests to a wholly different project and use on a variety of real world data sets rather than toys
fn better_than(f: impl Fn(&Vec<Bits>) -> Vec<u8>) {
    let item = make_item();
    // TODO: This is tuned to win at large numbers. How low can we get this and still reliably be better?
    let item = vec![item; 4];
    let bytes_tree = write(&item);
    let bytes_other = f(&item);
    assert!(bytes_tree.len() < bytes_other.len(), "Own: {}, other: {}", bytes_tree.len(), bytes_other.len());
    /*
    #[cfg(not(debug_assertions))]
    {
        // TODO: Deserialize
        let time_tree = bad_benchmark(|| { write(&item); });
        let time_other = bad_benchmark(|| { f(&item); });
        assert!(time_tree < time_other, "Own: {:?}, other: {:?}", time_tree, time_other);
    }
    */
}

#[test]
fn better_than_json() {
    better_than(|i| serde_json::to_vec(i).unwrap());
}

#[test]
fn better_than_message_pack() {
    use rmp_serde as rmps;
    use serde::Serialize;

    better_than(|i| {
        let mut buf = Vec::new();
        i.serialize(&mut rmps::Serializer::new(&mut buf)).unwrap();
        buf
    })
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
    pub struct _1 {
        a: u64,
    }

    round_trip_default::<_1>(4);
}

#[test]
fn large_structs() {
    #[derive(Read, Write, Default, Debug, PartialEq)]
    pub struct _14 {
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
    pub struct _15 {
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
    pub struct _16 {
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
    pub struct _17 {
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
