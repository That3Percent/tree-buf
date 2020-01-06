///! Static branches are used when the exact branching structure is known by the compiler.
///! The design allows for every function to be a const function. Everything is inlined so that
///! Monomorphization will write out code that is as though every function simply knows what to
///! do based on the context it was called in.
use super::*;
use std::marker::PhantomData;

pub struct StaticRootBranch;

impl StaticBranch for StaticRootBranch {
    #[inline(always)]
    fn children_in_array_context() -> bool {
        false
    }
}

pub struct OnlyBranch<T>(PhantomData<*const T>);

impl<T: StaticBranch> OnlyBranch<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: StaticBranch> StaticBranch for OnlyBranch<T> {
    #[inline(always)]
    fn children_in_array_context() -> bool {
        T::children_in_array_context()
    }
}