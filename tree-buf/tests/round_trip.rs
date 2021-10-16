use std::fmt::Debug;
use tree_buf::prelude::*;
mod common;
use common::*;
use std::collections::HashMap;
use tree_buf::encode_options;
use tree_buf::experimental::options;

// Create this namespace to hide the prelude. This is a check that the hygenics do not require any types from tree_buf to be imported
mod hide_namespace {
    use tree_buf::{Decode, Encode};
    #[derive(Encode, Decode, PartialEq, Debug, Clone)]
    pub struct Bits {
        pub f: f64,
        pub obj_array: Vec<Bobs>,
        pub extra: Option<Bobs>,
        pub s: Box<String>,
    }

    #[derive(Encode, Decode, PartialEq, Debug, Clone)]
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
fn broken_int() {
    round_trip(&75339u64, 4, 8);
}

#[test]
fn bools_root() {
    round_trip(&true, 1, 5);
    round_trip(&false, 1, 5);
}

#[test]
fn opts_root() {
    round_trip(&Some(true), 1, 9);
    round_trip(&Option::<bool>::None, 1, 3);
}

#[test]
fn bool_array() {
    round_trip(&vec![false, true, true, false, true, true, true, false, false, true, false, true], 6, 9);
}

#[test]
fn ints_root() {
    round_trip(&0u32, 1, 5);
    round_trip(&1u32, 1, 5);
    for i in 2..=127u32 {
        round_trip(&i, 2, 6);
    }
    for i in 128..=255u32 {
        round_trip(&i, 2, 6);
    }
    for i in 256..1024u32 {
        round_trip(&i, 3, 8);
    }
}

// Special case for 1 element array encodes root object
#[test]
fn array1() {
    round_trip(&vec![99u64], 3, 8);
    round_trip(&vec![1u64], 2, 7);
}

#[test]
fn int_vec() {
    round_trip(&vec![99u64, 100], 6, 10);
}

#[test]
fn float64_vec() {
    round_trip(&vec![0.99], 10, 16);
    round_trip(&vec![0.01, 0.02, 0.03, 0.04], 36, 65);
}

#[test]
fn float32_vec() {
    round_trip(&vec![0.99f32], 6, 14);
    round_trip(&vec![0.01f32, 0.02, 0.03, 0.04], 20, 38);
}

#[test]
fn lossy_f64_vec() {
    let mut data = Vec::new();
    for i in 0..50 {
        data.push(0.01 * i as f64);
    }
    let tolerance = -10;
    let options = encode_options! { options::LossyFloatTolerance(tolerance) };
    let binary = options::encode_with_options(&data, &options);
    assert_eq!(binary.len(), 104);
    let decoded = decode::<Vec<f64>>(&binary).unwrap();
    assert_eq!(data.len(), decoded.len());
    for (e, d) in data.iter().zip(decoded.iter()) {
        assert!((e - d).abs() <= 0.001);
    }

    // Show how much smaller this is than lossless
    let options = encode_options! { options::LosslessFloat };
    let binary = options::encode_with_options(&data, &options);
    assert_eq!(binary.len(), 376);

    // Show that this is much better than fixed, since this would be a minimum for exactly 0 schema overhead.
    assert_eq!(std::mem::size_of::<f64>() * data.len(), 400);
}

#[test]
fn nested_float_vec() {
    // FIXME: This increased in size with the fast_size_for change
    // See also 279e9860-d1f6-4a6e-a4bc-1a64c47b8370
    round_trip(&vec![vec![10.0, 11.0], vec![], vec![99.0]], 25, 32);
}

#[test]
fn array_tuple() {
    // FIXME: This increased in size with the fast_size_for change
    // See also 279e9860-d1f6-4a6e-a4bc-1a64c47b8370
    round_trip(&vec![vec![(1u32, 2u32), (3, 4), (5, 6)]], 16, 19);
}

#[test]
fn item() {
    let item = make_item();
    // FIXME: This increased in size with the fast_size_for change
    // See also 279e9860-d1f6-4a6e-a4bc-1a64c47b8370
    round_trip(&item, 145, 221);
}

#[test]
fn item_vec() {
    let item = make_item();
    let item = vec![item; 5];
    round_trip(&item, 379, 646);
}

#[test]
fn nullable_array() {
    round_trip(&vec![Some(1u32), Some(2)], 10, 14);
}

#[test]
fn visibility_modifiers() {
    #[derive(Default, Encode, Decode, Debug, PartialEq, Clone)]
    struct Inherited {
        a: u64,
    }

    #[derive(Default, Encode, Decode, Debug, PartialEq, Clone)]
    pub(crate) struct Crate {
        a: u64,
    }

    #[derive(Default, Encode, Decode, Debug, PartialEq, Clone)]
    pub struct Public {
        a: u64,
    }

    round_trip_default::<Inherited>(4, 8);
    round_trip_default::<Crate>(4, 8);
    round_trip_default::<Public>(4, 8);
}

#[test]
fn ignores() {
    use tree_buf::Ignore;
    round_trip(&Ignore, 1, 3);

    #[derive(Default, Encode, Decode, Debug, PartialEq, Clone)]
    struct X {
        i: Ignore,
    }

    let x = X { i: Ignore };
    round_trip(&x, 4, 6);

    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    enum E {
        A(Ignore),
        B(Ignore),
    }

    let e = E::A(Ignore);
    round_trip(&e, 4, 10);

    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    struct N {
        e: E,
    }

    let o = vec![N { e: E::A(Ignore) }, N { e: E::B(Ignore) }];
    round_trip(&o, 16, 18);
}

// TODO: Using Quickcheck and Arbitrary with quickcheck_derive.
#[test]
fn various_types() {
    round_trip_default::<u64>(1, 5);
    round_trip_default::<u32>(1, 5);
    round_trip_default::<u16>(1, 5);
    round_trip_default::<u8>(1, 5);
    round_trip_default::<(u64, u64)>(3, 9);
    round_trip_default::<(u64, u32)>(3, 9);
    round_trip_default::<f64>(1, 14);
    // See also: 84d15459-35e4-4f04-896f-0f4ea9ce52a9
    round_trip_default::<Vec<u32>>(1, 5);
    round_trip_default::<Option<Vec<u32>>>(1, 3);
    round_trip_default::<Option<u32>>(1, 3);
    round_trip_default::<Vec<Option<u32>>>(1, 5);
    round_trip_default::<String>(1, 6);
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
    #[derive(Encode, Decode, Default, Debug, PartialEq, Clone)]
    struct _1 {
        a: u64,
    }

    round_trip_default::<_1>(4, 8);
}

#[test]
fn large_structs() {
    #[derive(Encode, Decode, Default, Debug, PartialEq, Clone)]
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

    #[derive(Encode, Decode, Default, Debug, PartialEq, Clone)]
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

    #[derive(Encode, Decode, Default, Debug, PartialEq, Clone)]
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
    #[derive(Encode, Decode, Default, Debug, PartialEq, Clone)]
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

    round_trip_default::<_14>(44, 200);
    round_trip_default::<_15>(47, 214);
    round_trip_default::<_16>(50, 228);
    round_trip_default::<_17>(53, 242);
}

#[test]
fn map_0_root() {
    // See also: 84d15459-35e4-4f04-896f-0f4ea9ce52a9
    let data = HashMap::<u32, u32>::new();
    round_trip(&data, 2, 8);
}

#[test]
fn map_1_root() {
    let mut data = HashMap::new();
    data.insert("test".to_owned(), 5u32);
    round_trip(&data, 10, 21);
}

#[test]
fn map_n_root() {
    let mut data = HashMap::new();
    data.insert("test3".to_owned(), 5u32);
    data.insert("test2".to_owned(), 5);
    data.insert("test1".to_owned(), 0);
    round_trip(&data, None, None);
}

#[test]
fn maps_array() {
    let mut data = Vec::new();
    for i in 0..5u32 {
        let mut h = HashMap::new();
        h.insert(i, Vec::<u32>::new());
        h.insert(10, vec![10, 9, 8, 7]);
        data.push(h);
    }
    // Interestingly, the output size is not deterministic in this case.
    // It depends on whether the last key or value from iterating the HashMap is Default
    round_trip(&data, None, None);
}

#[test]
fn maps_void() {
    let mut data = Vec::new();
    for _ in 0..5 {
        let h = HashMap::<String, String>::new();
        data.push(h);
    }
    round_trip(&data, 10, 13);
}

#[test]
fn fixed_arrays() {
    round_trip(&[0u32, 1, 2, 3], 8, 10);
    round_trip(&[0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 6, 8);
}

// This failed to compile at one point when moving generics for EncoderArray out of associated type.
#[test]
fn enum_with_vec() {
    #[derive(Encode, Decode, Debug, PartialEq, Clone)]
    enum X {
        X(Vec<u64>),
    }

    round_trip(&X::X(vec![25, 30, 0, 0, 0]), 11, 21);
}

fn owned_vec(strs: Vec<&'static str>) -> Vec<String> {
    strs.iter().map(|s| String::from(*s)).collect()
}

#[test]
fn strings_using_dictionary() {
    let data = vec!["abcd", "abcd", "def", "abcd", "abcd", "abcd", ""];
    round_trip(&owned_vec(data), 21, 23);

    let data = vec!["abcd", "abcd", "abcd", "abcd", "abcd"];
    round_trip(&owned_vec(data), 13, 15);

    let data = vec!["abcd", "abcd", "abcd", "abcd", "abcd", "def", "def"];
    round_trip(&owned_vec(data), 17, 20);

    let data = vec!["abcd", "abcd", "abcd", "abcd", "abcd", "abcd", "def"];
    round_trip(&owned_vec(data), 17, 20);
}

#[test]
fn nested_strings_using_rle() {
    let data = (owned_vec(vec!["abc", "abc", "abc"]), owned_vec(vec!["def", "def", "def"]), 1u32);

    round_trip(&data, 26, 30);
}

#[test]
fn long_bool_runs() {
    let mut data = Vec::new();
    for i in 560..570 {
        for _ in 0..i {
            data.push(true);
        }
        data.push(false);
    }
    round_trip(&data, 36, 68);
}

#[test]
fn int_to_bool_nested() {
    let data = (vec![0u32, 0, 1, 1, 0], vec![0u32, 0, 0, 1, 1, 1, 1]);
    round_trip(&data, 11, 15);

    let data = vec![vec![0u32, 0, 1, 1, 0], vec![1u32, 1, 1, 1, 1, 1, 0], vec![1u32, 0, 0, 0, 0, 0, 1]];
    // FIXME: This increased in size with the fast_size_for change
    // See also 279e9860-d1f6-4a6e-a4bc-1a64c47b8370
    round_trip(&data, 14, 18);
}

#[test]
fn delta_prefix_var() {
    let data = vec![
        1_000_000_000u32,
        1_000_000_001,
        1_000_000_002,
        1_000_000_010,
        1_000_000_100,
        1_000_000_50,
        1_000_000_125,
        1_000_000_122,
        999_000_000,
        998_001_000,
        999_000_000,
        1,
        3_000_000_100,
        1_000,
    ];
    round_trip(&data, 49, 96);
}

#[test]
fn big_brotli_str() {
    let data = owned_vec(vec![
        "id,name,host_id,host_name,neighbourhood_group,neighbourhood,latitude,longitude,room_type,price,minimum_nights,number_of_reviews,last_review,reviews_per_month,calculated_host_listings_count,availability_365",
    "2818,Quiet Garden View Room & Super Fast WiFi,3159,Daniel,,Oostelijk Havengebied - Indische Buurt,52.36575,4.94142,Private room,59,3,277,2019-11-21,2.13,1,0",
    "20168,Studio with private bathroom in the centre 1,59484,Alexander,,Centrum-Oost,52.36509,4.89354,Private room,80,1,306,2019-12-05,2.57,2,198",
    "25428,Lovely apt in City Centre (w.lift) near Jordaan,56142,Joan,,Centrum-West,52.37297,4.88339,Entire home/apt,125,14,3,2019-05-11,0.13,2,71",
    "27886,\"Romantic, stylish B&B houseboat in canal district\",97647,Flip,,Centrum-West,52.38673,4.89208,Private room,155,2,206,2019-11-11,2.14,1,251",
    "28871,Comfortable double room,124245,Edwin,,Centrum-West,52.36719,4.89092,Private room,75,2,318,2019-11-23,2.81,3,134",
    "29051,Comfortable single room,124245,Edwin,,Centrum-West,52.36773,4.89151,Private room,55,2,467,2019-11-26,4.39,3,0",
    "31080,2-story apartment + rooftop terrace,133488,Nienke,,Zuid,52.35132,4.84838,Entire home/apt,219,3,32,2017-10-16,0.32,1,0",
    "41125,Amsterdam Center Entire Apartment,178515,Fatih,,Centrum-West,52.37891,4.88321,Entire home/apt,180,3,87,2019-07-08,0.79,1,34",
    "42970,Comfortable room@PERFECT location + 2 bikes,187580,,,Centrum-West,52.36781,4.89001,Hotel room,159,3,463,2019-11-05,4.11,2,121",
    "43109,Oasis in the middle of Amsterdam,188098,Aukje,,Centrum-West,52.37537,4.88932,Entire home/apt,210,3,821,2019-11-29,7.30,1,0",
    "43980,View into park / museum district (long/short stay),65041,Ym,,Zuid,52.35746,4.86124,Entire home/apt,100,30,61,2018-02-18,0.55,1,183",
    "46386,Cozy loft in central Amsterdam,207342,Joost,,De Pijp - Rivierenbuurt,52.35247,4.90825,Entire home/apt,150,3,3,2018-01-03,0.03,1,0",
    "47061,Charming apartment in old centre,211696,Ivar,,De Baarsjes - Oud-West,52.36799,4.87447,Entire home/apt,140,2,182,2019-06-19,1.62,1,0",
    "48076,Amsterdam Central and lot of space,219080,Franklin,,Centrum-West,52.38042,4.89453,Entire home/apt,270,7,201,2019-11-23,2.02,2,57",
    "49552,Multatuli Luxury Guest Suite in top location,225987,Joanna & MP,,Centrum-West,52.37925,4.89023,Entire home/apt,220,3,351,2019-12-05,3.16,1,0",
    "50515,\"Family Home (No drugs, smoking or parties)\",231864,Karin,,Bos en Lommer,52.37727,4.83925,Entire home/apt,120,4,15,2019-08-20,0.22,1,104",
    "50518,Perfect central Amsterdam apartment,231806,Nikki,,Westerpark,52.38201,4.87865,Entire home/apt,125,1,104,2019-11-24,1.20,1,3",
    "50523,B & B de 9 Straatjes (city center),231946,Raymond,,Centrum-West,52.36841,4.88413,Private room,115,2,279,2019-11-23,2.57,1,266",
    "50570,Bright Apartment - residential area,232321,Evert,,Bos en Lommer,52.37774,4.84891,Entire home/apt,90,4,157,2019-03-26,1.48,2,3",
    "52490,Amsterdam Aqua,185836,VictorLuke,,Oostelijk Havengebied - Indische Buurt,52.37005,4.93869,Private room,80,2,101,2019-11-25,0.91,1,16",
    "53067,Green studio at the attic of a townhouse,246493,DoJo,,De Pijp - Rivierenbuurt,52.35339,4.90064,Private room,87,5,348,2019-11-03,3.16,3,20",
    "53671,Nice room near centre with en suite bath,247822,Georg,,Westerpark,52.38905,4.88559,Private room,75,3,303,2019-11-23,2.87,1,257",
    "53692,Large quiet Studio with gardenview in hip area.,246493,DoJo,,De Pijp - Rivierenbuurt,52.35348,4.90049,Private room,60,1,308,2019-12-01,2.97,3,0",
    "55256,Luminous central room,260785,Lotte,,Centrum-Oost,52.37126,4.90351,Private room,86,1,177,2019-11-11,1.65,1,6",
    "55621,\"Fully equiped house, PIJP area = great\",262846,Bharati,,De Pijp - Rivierenbuurt,52.35386,4.89772,Entire home/apt,222,3,31,2019-07-28,0.28,1,7",
    "55703,groundfloor apartment with patio,263214,Arjan,,Bos en Lommer,52.37561,4.85819,Entire home/apt,250,3,3,2016-10-19,0.06,1,341",
    "55807,\"Greatly located, cozy atmosphere\",263844,Rai & Clo,,De Baarsjes - Oud-West,52.36966,4.86203,Private room,60,2,156,2019-06-23,1.41,1,115",
    "55868,Apartment near Museumplein (centre),264178,Cornelie,,Zuid,52.35613,4.88515,Entire home/apt,149,4,94,2019-10-26,0.86,1,4",
    "56879,\"86 m2, city centre & lovely view\",270282,Linda & Theo,,Centrum-West,52.38453,4.89255,Entire home/apt,112,28,1,2012-07-27,0.01,1,301",
    "58211,En Suite accommodation in a monumental canal house,278253,Marcel,,Centrum-West,52.36916,4.88445,Private room,220,3,105,2019-12-01,1.16,2,0",
    "62015,\"Charming, beautifully & sunny place\",301340,Jessica,,Oud-Oost,52.36445,4.93124,Entire home/apt,109,2,30,2019-11-24,0.28,1,0",
    "62801,Very nice appartment ALL of januari,306117,Jan,,Bos en Lommer,52.38459,4.85463,Entire home/apt,750,30,0,,,1,365",
    "63872,HOLIDAY SPECIAL - AMSTERDAM ARTIST LOFT,312121,Morgan,,Centrum-Oost,52.37117,4.90948,Entire home/apt,170,2,123,2019-08-19,1.16,1,19",
    "64736,Luxury Houseboat,306192,Conny,,De Baarsjes - Oud-West,52.36348,4.86783,Entire home/apt,189,3,103,2019-08-06,1.35,1,0",
    "64769,Unique 3 bedroom house in Centre,312863,,,Centrum-Oost,52.36225,4.902,Entire home/apt,450,4,27,2019-01-05,0.26,1,0",
    "67841,Amsterdam - The Pijp Apartment 1A,335166,Dene,,De Pijp - Rivierenbuurt,52.35472,4.89324,Entire home/apt,100,2,16,2019-08-12,0.21,2,0",
    "69042,Cozy comfortable Studio | At the Canals | Vondelpark!,344312,Kjetil,,Centrum-West,52.37353,4.87691,Hotel room,108,3,153,2019-11-11,1.46,2,0",
    "70598,Attic room with doublebed available,339333,Nadira,,Noord-West,52.41433,4.92014,Private room,53,1,103,2019-12-08,2.27,1,11",
    "73208,Centre Museum Quarter Apt Roof Deck,381900,Vikki,,Zuid,52.35935,4.87672,Entire home/apt,220,5,62,2019-07-06,0.58,1,0",
    "73917,B28 Unique Houseboat  Herengracht,387205,Edwin,,Centrum-West,52.37894,4.8906,Entire home/apt,175,3,94,2019-08-19,1.18,1,240",
    "75382,Garden Suite (Website hidden by Airbnb),399879,Tina,,Oud-Oost,52.36482,4.92762,Private room,105,3,238,2019-11-21,2.28,3,62",
    "75444,Cottage Room- Completely Private,399879,Tina,,Oud-Oost,52.36551,4.92848,Private room,70,3,304,2019-11-20,2.93,3,0",
    "76668,\"studio INN, bright and spacious\",409579,Guido,,Westerpark,52.38898,4.89043,Hotel room,145,3,60,2019-12-01,1.43,1,207",
    "80635,TOP LOCATED Canalhouse B&B Jordaan,436145,Riks,,Centrum-West,52.37876,4.89264,Private room,105,2,215,2019-11-30,2.03,3,0",
    "82482,The Backroom - Central private appt,186729,Shawna,,Centrum-West,52.37026,4.88003,Entire home/apt,95,1,811,2019-11-22,7.86,1,173",
    "82748,Bright apt in central Amsterdam,450453,Rudolf,,Centrum-West,52.37307,4.89343,Entire home/apt,249,3,130,2019-08-22,1.23,2,0",
    ]);

    // Without Brotli:
    //round_trip(&data, 6382, 6459);

    // With Brotli:
    round_trip(&data, 2347, 2405);
}

// TODO: Use coverage marks to ensure all types are used
// https://ferrous-systems.com/blog/coverage-marks/

/*
// This was useful for narrowing down a subset of a broken compressor.
// It may be useful in the future
#[test]
#[ignore]
fn broken_gorilla() {
    use rand::Rng;
    use std::convert::TryInto as _;
    use tree_buf::internal::encodings::gorilla;

    let data = std::fs::read("C:\\git\\floats.dat").unwrap();
    let mut offset = 0;
    let mut values = Vec::new();
    while offset < data.len() {
        let val = (&data[offset..(offset + 8)]).try_into().unwrap();
        offset += 8;
        let f = f64::from_le_bytes(val);
        values.push(f);
    }

    fn attempt(values: &[f64], min: usize, max: usize) -> bool {
        let values = &values[min..max];
        std::panic::catch_unwind(|| {
            let mut bytes = Vec::new();
            gorilla::compress(values.iter().copied(), &mut bytes).unwrap();
            let out: Vec<f64> = gorilla::decompress(&bytes[..]).unwrap();
            assert_eq!(values, &out[..]);
            assert!(bytes.len() == tree_buf::internal::gorilla::size_for(values.iter().copied()).unwrap());
        })
        .is_ok()
    }

    let mut min = 0;
    let mut max = values.len();

    let mut rng = rand::thread_rng();
    for _ in 0..10000 {
        let try_min = rng.gen_range(min, max);
        let try_max = rng.gen_range(try_min + 1, max + 1);
        if try_min == min && try_max == max {
            continue;
        }
        if !attempt(&values[..], try_min, try_max) {
            min = try_min;
            max = try_max;
        }
    }
}
*/
