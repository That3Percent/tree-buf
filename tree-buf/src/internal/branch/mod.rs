use crate::prelude::*;
use crate::internal::encodings::varint::{decode_prefix_varint, decode_suffix_varint};
use std::fmt::Debug;
use std::collections::HashMap;

pub mod dyn_branch;
pub mod static_branch;
pub use dyn_branch::*;
pub use static_branch::*;


pub trait StaticBranch : 'static {
    fn children_in_array_context() -> bool;
    fn self_in_array_context() -> bool;
}

