use crate::branch::*;
use std::any::Any;
use crate::primitive::*;

pub struct Context<'a> {
    branch_map: BranchMap<'a, Box<dyn Any>>,
}

impl Context<'_> {
    pub fn get_writer<'a, 'b, T: Primitive>(&'a mut self, branch: &'b Branch) -> &'a mut PrimitiveBuffer<T> {
        todo!();
    }
    // TODO: Separate Reader/Writer
    // TODO: Consider separating result for mismatched typed vs completely missing.

    pub fn get_reader<'a, 'b, T: Primitive>(&'a mut self, branch: &'b Branch) -> Option<&'a mut PrimitiveBuffer<T>> {
        todo!();
    }

    pub fn new() -> Self {
        Self {
            branch_map: BranchMap::new()
        }
    }
}
