pub mod dyn_branch;
pub mod static_branch;
pub use dyn_branch::*;
pub use static_branch::*;


pub trait StaticBranch : 'static {
    fn children_in_array_context() -> bool;
}