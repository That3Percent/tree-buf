macro_rules! impl_fixed {
    ($(($size:expr, $namespace:ident)),+) => {
        $(mod $namespace {
            use crate::prelude::*;
            use std::mem::{MaybeUninit};
            // This transmute is a (seriously unsafe) version
            // of std::mem::transmute which allows unmatched sizes
            // This is needed here because of a compiler issue preventing
            // std::mem::transmute to be used: https://github.com/rust-lang/rust/issues/47966
            use transmute::transmute;

            #[cfg(feature = "write")]
            impl<T: Writable> Writable for [T; $size] {
                type WriterArray = ArrayWriter<T::WriterArray>;
                fn write_root(&self, stream: &mut impl WriterStream) -> RootTypeId {
                    profile!("write_root");
                    match self.len() {
                        0 => RootTypeId::Array0,
                        1 => {
                            stream.write_with_id(|stream| (&self[0]).write_root(stream));
                            RootTypeId::Array1
                        }
                        _ => {
                            // TODO: Seems kind of redundant to have both the array len,
                            // and the bytes len. Though, it's not for obvious reasons.
                            // Maybe sometimes we can infer from context. Eg: bool always
                            // requires the same number of bits per item
                            write_usize(self.len(), stream);

                            // TODO: When there are types that are already
                            // primitive (eg: Vec<f64>) it doesn't make sense
                            // to buffer at this level. Specialization may
                            // be useful here.
                            //
                            // TODO: See below, and just call buffer on the vec
                            // and flush it!
                            let mut writer = T::WriterArray::default();
                            for item in self.iter() {
                                writer.buffer(item);
                            }

                            stream.write_with_id(|stream| writer.flush(stream));

                            RootTypeId::ArrayN
                        }
                    }
                }
            }

            // Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
            #[cfg(feature = "read")]
            impl<T: Readable + Sized> Readable for [T; $size] where ReadError : From<<T::ReaderArray as ReaderArray>::Error> {
                type ReaderArray = ArrayReader<T::ReaderArray>;
                fn read(sticks: DynRootBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
                    profile!("Readable::read");
                    match sticks {
                        DynRootBranch::Array0 => {
                            if $size != 0 {
                                return Err(ReadError::SchemaMismatch);
                            } else {
                                let data: [MaybeUninit<T>; $size] = unsafe {
                                    MaybeUninit::uninit().assume_init()
                                };
                                // Safety - we can only get here if size = 0, so
                                // it's an empty array of uninit and doesn't need to
                                // be initialized
                                Ok(unsafe { transmute(data) })
                            }
                        },
                        DynRootBranch::Array1(inner) => {
                            if $size != 1 {
                                return Err(ReadError::SchemaMismatch);
                            }
                            let inner = T::read(*inner, options)?;

                            let mut data: [MaybeUninit<T>; $size] = unsafe {
                                MaybeUninit::uninit().assume_init()
                            };
                            data[0] = MaybeUninit::new(inner);
                            // Safety: Verified that size = 1, so the only element has
                            // been initialized
                            Ok(unsafe { transmute(data) })
                        }
                        DynRootBranch::Array { len, values } => {
                            if len != $size {
                                return Err(ReadError::SchemaMismatch);
                            }
                            let mut reader = T::ReaderArray::new(values, options)?;
                            let mut data: [MaybeUninit<T>; $size] = unsafe {
                                MaybeUninit::uninit().assume_init()
                            };

                            for elem in &mut data[..] {
                                *elem = MaybeUninit::new(reader.read_next()?);
                            }

                            Ok(unsafe { transmute(data) })
                        }
                        _ => Err(ReadError::SchemaMismatch),
                    }
                }
            }


            #[cfg(feature = "write")]
            #[derive(Debug, Default)]
            pub struct ArrayWriter<T> {
                values: T,
            }

            #[cfg(feature = "read")]
            pub struct ArrayReader<T> {
                values: T,
            }

            #[cfg(feature = "write")]
            impl<T: Writable> WriterArray<[T; $size]> for ArrayWriter<T::WriterArray> {
                fn buffer<'a, 'b: 'a>(&'a mut self, value: &'b [T; $size]) {
                    // TODO: Consider whether buffer should actually just
                    // do something non-flat, (like literally push the Vec<T> into another Vec<T>)
                    // and the flattening could happen later at flush time. This may reduce memory cost.
                    // Careful though.
                    // I feel though that somehow this outer buffer type
                    // could fix the specialization problem above for single-vec
                    // values.
                    for item in value.iter() {
                        self.values.buffer(item);
                    }
                }
                fn flush(self, stream: &mut impl WriterStream) -> ArrayTypeId {
                    profile!("flush");
                    let Self { values } = self;
                    write_usize($size, stream);
                    if $size != 0 {
                        stream.write_with_id(|stream| values.flush(stream));
                    }

                    ArrayTypeId::ArrayFixed
                }
            }

            #[cfg(feature = "read")]
            impl<T: ReaderArray> ReaderArray for ArrayReader<T> {
                type Read = [T::Read; $size];
                type Error = T::Error;
                fn new(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
                    profile!("ReaderArray::new");

                    match sticks {
                        DynArrayBranch::ArrayFixed { len, values } => {
                            if len != $size {
                                return Err(ReadError::SchemaMismatch);
                            }
                            let values = T::new(*values, options)?;
                            Ok(ArrayReader { values })
                        }
                        _ => Err(ReadError::SchemaMismatch),
                    }
                }
                fn read_next(&mut self) -> Result<Self::Read, Self::Error> {
                    let mut data: [MaybeUninit<T::Read>; $size] = unsafe {
                        MaybeUninit::uninit().assume_init()
                    };

                    for elem in &mut data[..] {
                        *elem = MaybeUninit::new(self.values.read_next()?);
                    }

                    // Safety - all elements initialized in loop
                    Ok(unsafe { transmute(data) })
                }
            }

        })+
    };
}

impl_fixed!(
    // TODO: Re-add these, consider the changes that have to occur in ReaderArray.
    // It should be relatively simple, Add Eg: ArrayFixed0 variant
    //(0, _0),
    //(1, _1),
    (2, _2),
    (3, _3),
    (4, _4),
    (5, _5),
    (6, _6),
    (7, _7),
    (8, _8),
    (9, _9),
    (10, _10),
    (11, _11),
    (12, _12),
    (13, _13),
    (14, _14),
    (15, _15),
    (16, _16),
    (32, _32),
    (64, _64),
    (128, _128),
    (256, _256),
    (512, _512)
);
