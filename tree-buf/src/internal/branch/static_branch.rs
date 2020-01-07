///! Static branches are used when the exact branching structure is known by the compiler.
///! The design allows for every function to be a const function. Everything is inlined so that
///! Monomorphization will write out code that is as though every function simply knows what to
///! do based on the context it was called in.
use super::*;
use std::marker::PhantomData;

#[derive(Copy, Clone)]
pub struct NonArrayBranch;

impl StaticBranch for NonArrayBranch {
    #[inline(always)]
    fn in_array_context() -> bool {
        false
    }
}
