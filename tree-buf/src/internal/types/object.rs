use crate::prelude::*;
use std::marker::PhantomData;

// TODO: The interaction between Default and Missing here may be dubious.
// What it will ultimately infer is that the struct exists, but that all it's
// fields should also come up missing. Where this gets really sketchy though
// is that there may be no mechanism to ensure that none of it's fields actually
// do come up missing in the event of a name collision. I think what we actually
// want is to try falling back to the owning struct default implementation instead,
// but that would require Default on too much. Having the branch type be a part
// of the lookup somehow, or have missing be able to cancel the branch to something bogus may help.
//
// Ammendment to previous. This comment is somewhat out of date, now that Missing isn't really implemented,
// and that the schema match has been moved to one place.
#[derive(Copy, Clone, Default, Debug)]
pub struct Object;

impl Primitive for Object {
    fn id() -> PrimitiveId {
        PrimitiveId::Object
    }
}


// TODO: Performance - remove the need to allocate vec here.
impl BatchData for Object {
    fn write_batch(_items: &[Self], _bytes: &mut Vec<u8>) { }
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        debug_assert_eq!(bytes.len(), 0);
        Vec::new()
    }
}

pub struct ObjectBranch<T> {
    name: &'static str,
    _marker: PhantomData<*const T>
}

impl<T: StaticBranch> ObjectBranch<T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            _marker: PhantomData,
        }
    }
}

impl <T: StaticBranch> StaticBranch for ObjectBranch<T> {
    #[inline(always)]
    fn children_in_array_context() -> bool {
        Self::self_in_array_context()
    }
    #[inline(always)]
    fn self_in_array_context() -> bool {
        T::children_in_array_context()
    }
    #[inline(always)]
    fn name(&self) -> Option<&str> {
        Some(self.name)
    }
}