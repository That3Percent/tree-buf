use crate::prelude::*;

pub trait Missing {
    fn missing<T: Primitive>(&self, branch: &BranchId) -> Result<T, Error>;
}

pub struct ErrOnMissing;
impl Missing for ErrOnMissing {
    #[inline(always)]
    fn missing<T: Primitive>(&self, branch: &BranchId) -> Result<T, Error> {
        Err(Error::Missing {
            branch: format!("{:?}", branch), // TODO: This no longer carries the whole parent name, which would be desirable
            id: T::id(),
        })
    }
}

pub struct DefaultOnMissing;
impl Missing for DefaultOnMissing {
    #[inline(always)]
    fn missing<T: Default>(&self, _branch: &BranchId) -> Result<T, Error> {
        Ok(T::default())
    }
}

