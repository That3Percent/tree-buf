use crate::context::*;
use crate::branch::*;
use crate::missing::*;
use crate::error::*;

pub trait Writer {
    /// The writer can assume that the branch is already correct.
    /// It may ask for the writer it needs for it's data type at the given branch.
    fn write(&self, context: &mut Context<'_>, branch: &Branch<'_>);
}

pub trait Reader {
    fn read(context: &mut Context, branch: &Branch, missing: &impl Missing) -> Result<Self, Error> where Self : Sized;
}
