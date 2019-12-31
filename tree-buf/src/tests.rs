use crate::prelude::*;

#[derive(PartialEq, Debug)]
struct Item {
    int: u32,
    obj_array: Vec<Bob>,
    extra: Option<Bob>,
}

#[derive(Debug)]
struct ItemWriter {
    _struct: <Struct as Writable>::Writer,
    int: <u32 as Writable>::Writer,
    obj_array: <Vec<Bob> as Writable>::Writer,
    extra: <Option<Bob> as Writable>::Writer,
}

struct ItemReader {
    _struct: <Struct as Readable>::Reader,
    int: <u32 as Readable>::Reader,
    obj_array: <Vec<Bob> as Readable>::Reader,
    extra: <Option<Bob> as Readable>::Reader,
}

impl Writer for ItemWriter {
    type Write=Item;
    fn new() -> Self {
        Self {
            _struct: Writer::new(),
            int: Writer::new(),
            obj_array: Writer::new(),
            extra: Writer::new(),
        }
    }
    fn write(&mut self, value: &Item) {
        self._struct.write(&Struct);
        self.int.write(&value.int);
        self.obj_array.write(&value.obj_array);
        self.extra.write(&value.extra);
    }
    fn flush(&self, branch: &BranchId<'_>, bytes: &mut Vec<u8>) {
        let own_id = bytes.len();
        self._struct.flush(branch, bytes);

        let int = BranchId { name: "int", parent: own_id };
        self.int.flush(&int, bytes);

        let obj_array = BranchId { name: "obj_array", parent: own_id };
        self.obj_array.flush(&obj_array, bytes);

        let extra = BranchId { name: "extra", parent: own_id };
        self.extra.flush(&extra, bytes);
    }
}

impl Reader for ItemReader {
    type Read=Item;
    fn read(&mut self) -> Self::Read  {
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

        let obj_array = BranchId { name: "obj_array", parent: own_id };
        let obj_array = Reader::new(sticks, &obj_array);

        let extra = BranchId { name: "extra", parent: own_id };
        let extra = Reader::new(sticks, &extra);

        Self {
            _struct,
            int,
            obj_array,
            extra,
        }
    }
}

impl Writable for Item {
    type Writer=ItemWriter;
}

impl Readable for Item {
    type Reader=ItemReader;
}


#[derive(PartialEq, Debug)]
pub struct Bob {
    one: Vec<u32>,
}

#[derive(Debug)]
pub struct BobWriter {
    _struct: <Struct as Writable>::Writer,
    one: <Vec<u32> as Writable>::Writer,
}

pub struct BobReader {
    _struct: <Struct as Readable>::Reader,
    one: <Vec<u32> as Readable>::Reader,
}

impl Writer for BobWriter {
    type Write=Bob;
    fn new() -> Self {
        Self {
            _struct: <Struct as Writable>::Writer::new(),
            one: <Vec<u32> as Writable>::Writer::new(),
        }
    }
    fn write(&mut self, value: &Self::Write) {
        self._struct.write(&Struct);
        self.one.write(&value.one);
    }
    fn flush(&self, branch: &BranchId<'_>, bytes: &mut Vec<u8>) {
        let own_id = bytes.len();
        self._struct.flush(branch, bytes);

        let one = BranchId { name: "one", parent: own_id };
        self.one.flush(&one, bytes);
    }
}

impl Reader for BobReader {
    type Read=Bob;
    fn new(sticks: &Vec<Stick>, branch: &BranchId) -> Self {
        let own_id = branch.find_stick(sticks).unwrap().start; // TODO: Error handling
        let _struct = Reader::new(sticks, branch);

        let one = BranchId { name: "one", parent: own_id };
        let one = Reader::new(sticks, &one);

        Self {
            _struct,
            one
        }
    }
    fn read(&mut self) -> Self::Read {
        self._struct.read();
        Self::Read {
            one: self.one.read(),
        }
    }
}

impl Writable for Bob {
    type Writer=BobWriter;
}

impl Readable for Bob {
    type Reader=BobReader;
}


#[test]
fn round_trip() {
    let item = Item {
        int: 5,
        extra: Some(Bob {
            one: vec! { 99 },
        }),
        obj_array: vec! {
            Bob {
                one: vec! { 3, 2, 1, 0 },
            },
            Bob {
                one: vec! { },
            },
            Bob {
                one: vec! { 20, 20, 20, 20, 20, 20, 20 }
            }
        },
    };
    let bytes = crate::write(&item);
    let result = crate::read(&bytes);
    assert_eq!(Ok(item), result);
}