use crate::prelude::*;

pub trait Missing {
    fn missing<T: Primitive>(&self, branch: &Branch) -> Result<T, Error>;
}

pub struct ErrOnMissing;
impl Missing for ErrOnMissing {
    #[inline(always)]
    fn missing<T: Primitive>(&self, branch: &Branch) -> Result<T, Error> {
        Err(Error::Missing {
            branch: format!("{:?}", branch),
            id: T::id(),
        })
    }
}

pub struct DefaultOnMissing;
impl Missing for DefaultOnMissing {
    #[inline(always)]
    fn missing<T: Default>(&self, _branch: &Branch) -> Result<T, Error> {
        Ok(T::default())
    }
}

