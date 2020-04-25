#[derive(Default, Debug)]
pub(crate) struct Unowned<T: ?Sized> {
    _marker: std::marker::PhantomData<*const T>,
}


impl<T> Copy for Unowned<T> {}
impl<T> Clone for Unowned<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Unowned<T> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}
unsafe impl<T> Send for Unowned<T> {}