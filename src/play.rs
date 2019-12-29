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

impl Writer for Item {
    fn write(&self, context: &mut Context<'_>, branch: &Branch<'_>) {
        Struct.write(context, branch);
        self.int.write(context, &branch.child("int"));
        self.obj_array.write(context, &branch.child("obj_array"));
    }
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

impl Writer for Bob {
    fn write(&self, context: &mut Context<'_>, branch: &Branch<'_>) {
        Struct.write(context, branch);
        self.one.write(context, &branch.child("one"));
    }
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
                one: vec! { 3, 2, 1 },
            }
        },
    };
    let bytes = crate::write(&item);
    let result = crate::read(&bytes);
    assert_eq!(Ok(item), result);
}