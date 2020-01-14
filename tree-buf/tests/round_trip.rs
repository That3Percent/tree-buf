use tree_buf::prelude::*;
use tree_buf::{Readable, Writable};
use std::fmt::Debug;
#[cfg(not(debug_assertions))]
use std::time::{Instant, Duration};

// Create this namespace to hide the prelude. This is a check that the hygenics do not require any types from tree_buf to be imported
mod hide_namespace {
    use tree_buf::{Read, Write};
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize)]
    #[derive(Read, Write, PartialEq, Debug, Clone)]
    pub struct Bits {
        pub f: f64,
        pub obj_array: Vec<Bobs>,
        pub extra: Option<Bobs>,
        pub s: Box<String>,
    }

    #[derive(Serialize, Deserialize)]
    #[derive(Read, Write, PartialEq, Debug, Clone)]
    pub struct Bobs {
        pub one: Vec<u32>,
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
            Bobs {
                one: vec![],
                tup: (2.2, 200.3),
            },
            Bobs {
                one: vec![20, 20, 20, 20, 20, 20, 20],
                tup: (0.0, 200.4),
            },
        ],
    }
}

fn round_trip_default<T: for<'a> Default + Readable + for<'a> Writable<'a> + Debug + PartialEq>() {
    let data = T::default();
    round_trip(&data);
}

fn round_trip<'a, 'b : 'a, T: Readable + Writable<'a> + Debug + PartialEq>(value: &'b T) {
    let bytes = write(value);
    let result = read(&bytes);
    match result {
        Ok(parsed) => assert_eq!(value, &parsed),
        Err(e) => assert!(false, "{}", e),
    }
}

#[test]
fn round_trip_item() {
    let item = make_item();
    round_trip(&item);
}


#[test]
fn round_trip_vec() {
    let item = make_item();
    let item = vec![item; 5];
    round_trip(&item);
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
    let item = vec![item; 30];
    let bytes_tree = write(&item);
    let bytes_other = f(&item);
    assert!(bytes_tree.len() < bytes_other.len(), "Own: {}, other: {}", bytes_tree.len(), bytes_other.len());
    #[cfg(not(debug_assertions))]
    {
        // TODO: Deserialize
        let time_tree = bad_benchmark(|| { write(&item); });
        let time_other = bad_benchmark(|| { f(&item); });
        assert!(time_tree < time_other, "Own: {:?}, other: {:?}", time_tree, time_other);
    }
}

#[test]
fn better_than_json() {
    better_than(|i| { serde_json::to_vec(i).unwrap() });
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

#[test]
fn size_check() {
    // TODO: Compare output size of file and time to encode and decode as compared to a variety of other formats.
    let item = make_item();
    //let item = vec![item; 5];
    let bytes = write(&item);

    // Assert a specific size. If we get a number above this size, that's a fail.
    // If we add compression and achieve lower, we can ratchet the number down.
    assert_eq!(bytes.len(), 152);
}

// TODO: Using Quickcheck and Arbitrary with quickcheck_derive.
#[test]
fn various_types() {
    round_trip_default::<u64>();
    round_trip_default::<u32>();
    round_trip_default::<u16>();
    round_trip_default::<u8>();
    round_trip_default::<(u64, u64)>();
    round_trip_default::<(u64, u32)>();
    round_trip_default::<f64>();
    round_trip_default::<Vec<u32>>();
    round_trip_default::<Option<Vec<u32>>>();
    round_trip_default::<Option<u32>>();
    round_trip_default::<Vec<Option<u32>>>();
    round_trip_default::<String>();
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

    round_trip_default::<_14>();
    round_trip_default::<_15>();
    round_trip_default::<_16>();
    round_trip_default::<_17>();
}
