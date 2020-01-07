use crate::prelude::*;

impl <T0: Writable> Writable for (T0,) {
    type Writer=(T0::Writer,);
}

impl <T0: Readable> Readable for (T0,) {
    type Reader=(T0::Reader,);
}

impl <T0: Writer> Writer for (T0,) {
    type Write=(T0::Write,);

    fn write(&mut self, value: &Self::Write) {
        let (t0,) = self;
        t0.write(&value.0);
    }
    fn flush<ParentBranch: StaticBranch>(self, branch: ParentBranch, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) {
        PrimitiveId::Tuple { num_fields: 1}.write(bytes);
        let (t0,) = self;
        t0.flush(branch, bytes, lens);
    }
    fn new() -> Self {
        (
            T0::new(),
        )
    }
}

impl <T0: Reader> Reader for (T0,) {
    type Read=(T0::Read,);
    fn new<ParentBranch: StaticBranch>(sticks: DynBranch<'_>, branch: ParentBranch) -> Self {
        match sticks {
            DynBranch::Tuple { mut children } => {
                if children.len() != 1 {
                    todo!("schema mismatch");
                }
                let mut children = children.drain(..);
                (
                    T0::new(children.next().unwrap(), branch),
                )
            },
            _ => todo!("schema mismatch")
        }
    }
    fn read(&mut self) -> Self::Read {
        let (t0,) = self;

        (
            t0.read(),
        )
    }
}