use crate::branch::*;
use crate::primitive::*;
use crate::missing::*;
use crate::reader_writer::*;
use crate::error::*;
use crate::context::*;


#[derive(PartialEq, Debug)]
struct Item {
    int: u32,
    obj_array: Vec<Bob>,
}

#[derive(Debug)]
struct ItemWriter {
    _struct: <Struct as Writable>::Writer,
    int: <u32 as Writable>::Writer,
    obj_array: <Vec<Bob> as Writable>::Writer,
}

impl Writer for ItemWriter {
    type Write=Item;
    fn new() -> Self {
        Self {
            _struct: Writer::new(),
            int: Writer::new(),
            obj_array: Writer::new(),
        }
    }
    fn write(&mut self, value: &Item) {
        self._struct.write(&Struct);
        self.int.write(&value.int);
        self.obj_array.write(&value.obj_array);
    }
    fn flush(&self, branch: &BranchId<'_>, bytes: &mut Vec<u8>) {
        let own_id = bytes.len();
        self._struct.flush(branch, bytes);

        let int = BranchId { name: "int", parent: own_id };
        self.int.flush(&int, bytes);

        let obj_array = BranchId { name: "obj_array", parent: own_id };
        self.obj_array.flush(&obj_array, bytes);
    }
}

impl Writable for Item {
    type Writer=ItemWriter;
}

impl Reader for Item {
    fn read(context: &mut Context, branch: &Branch, missing: &impl Missing) -> Result<Self, Error> where Self : Sized {
        Struct::read(context, branch, missing)?;
        Ok(Self {
            int: Reader::read(context, &branch.child("int"), missing)?,
            obj_array: Reader::read(context, &branch.child("obj_array"), missing)?,
        })
    }
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

impl Writable for Bob {
    type Writer=BobWriter;
}

impl Reader for Bob {
    fn read(context: &mut Context, branch: &Branch, missing: &impl Missing) -> Result<Self, Error> where Self : Sized {
        Struct::read(context, branch, missing)?;
        Ok(Self {
            one: Reader::read(context, &branch.child("one"), missing)?
        })
    }
}



pub fn test() {
    let item = Item {
        int: 5,
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
    dbg!(bytes.len());
    let result = crate::read(&bytes);
    assert_eq!(Ok(item), result);
}

#[cfg(test)]
#[test]
fn play_test() {
    test();
}