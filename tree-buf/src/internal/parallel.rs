use crate::prelude::*;
use rayon;

// TODO: Have a way to not use rayon when the operation is considered 'trivial'
#[inline(always)]
pub fn parallel<A: Send, B: Send>(
    a: impl FnOnce() -> A + Send,
    b: impl FnOnce() -> B + Send,
    options: &impl DecodeOptions) -> (A, B) {
    if options.parallel() {
        rayon::join(a, b)
    } else {
        (a(), b())
    }
}