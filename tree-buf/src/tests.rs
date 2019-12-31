use crate::prelude::*;
use std::fmt::Debug;
use tree_buf_macros::Write;
use crate as tree_buf; // This warns about being unused, but it's used in the macro.

#[derive(PartialEq, Debug, Clone, Write)]
struct Item {
    int: u32,
    obj_array: Vec<Bob>,
    extra: Option<Bob>,
}



struct ItemReader {
    _struct: <Struct as Readable>::Reader,
    int: <u32 as Readable>::Reader,
    obj_array: <Vec<Bob> as Readable>::Reader,
    extra: <Option<Bob> as Readable>::Reader,
}


impl Reader for ItemReader {
    type Read = Item;
    fn read(&mut self) -> Self::Read {
        self._struct.read();
        Item {
            int: self.int.read(),
            obj_array: self.obj_array.read(),
            extra: self.extra.read(),
        }
    }
    fn new(sticks: &Vec<Stick>, branch: &BranchId) -> Self {
        let own_id = branch.find_stick(sticks).unwrap().start; // TODO: Error handling

        let _struct = Reader::new(sticks, branch);
        let int = BranchId { name: "int", parent: own_id };
        let int = Reader::new(sticks, &int);

        let obj_array = BranchId {
            name: "obj_array",
            parent: own_id,
        };
        let obj_array = Reader::new(sticks, &obj_array);

        let extra = BranchId { name: "extra", parent: own_id };
        let extra = Reader::new(sticks, &extra);

        Self { _struct, int, obj_array, extra }
    }
}


impl Readable for Item {
    type Reader = ItemReader;
}

#[derive(PartialEq, Debug, Clone, Write)]
struct Bob {
    one: Vec<u32>,
}

struct BobReader {
    _struct: <Struct as Readable>::Reader,
    one: <Vec<u32> as Readable>::Reader,
}


impl Reader for BobReader {
    type Read = Bob;
    fn new(sticks: &Vec<Stick>, branch: &BranchId) -> Self {
        let own_id = branch.find_stick(sticks).unwrap().start; // TODO: Error handling
        let _struct = Reader::new(sticks, branch);

        let one = BranchId { name: "one", parent: own_id };
        let one = Reader::new(sticks, &one);

        Self { _struct, one }
    }
    fn read(&mut self) -> Self::Read {
        self._struct.read();
        Self::Read { one: self.one.read() }
    }
}

impl Readable for Bob {
    type Reader = BobReader;
}

fn make_item() -> Item {
    Item {
        int: 5,
        extra: Some(Bob { one: vec![99] }),
        obj_array: vec![
            Bob { one: vec![3, 2, 1, 0] },
            Bob { one: vec![] },
            Bob {
                one: vec![20, 20, 20, 20, 20, 20, 20],
            },
        ],
    }
}

fn round_trip<T: Readable + Writable + Debug + PartialEq>(value: &T) {
    let bytes = crate::write(value);
    let result = crate::read(&bytes);
    match result {
        Ok(parsed) => assert_eq!(value, &parsed),
        _ => assert!(false),
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
