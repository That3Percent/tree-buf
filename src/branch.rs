use std::fmt::{Debug, Formatter, Error, Write};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::{DefaultHasher};
use crate::prelude::*;

// TODO: In file, store branch as -
// Prev branch (id, # in file), name, type, data ptr

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Branch<'a> {
    namespace: &'static str,
    // FIXME: Add data type id - array, struct, particular primitive, etc.
    prev: Option<&'a Branch<'a>>,
}

impl<'a> Debug for Branch<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if let Some(prev) = self.prev {
            prev.fmt(f)?;
            f.write_char(':')?;
        }
        
        f.write_str(self.namespace)
    }
}

impl<'a> Branch<'a> {
    pub fn root() ->  Self {
        Self {
            namespace: "",
            prev: None,
        }
    }
    pub fn child(&'a self, namespace: &'static str) -> Self {
        Self {
            namespace,
            prev: Some(&self),
        }
    }
    
    pub fn hash_128(&self) -> u128 {
        // FIXME: This uses u64
        let mut hasher = DefaultHasher::default();
        self.hash(&mut hasher);
        hasher.finish().into()
    }
}


pub struct BranchId<'a> {
    pub name: &'a str,
    // The parent is just the start byte of the parent branch.
    // Every branch must at least write it's primitive id, so these are guaranteed to be unique.
    pub parent: usize,
}

impl<'a> BranchId<'a> {
    pub(crate) fn flush(&self, bytes: &mut Vec<u8>) {
        // Parent, Name length, name bytes
        self.parent.write(bytes);
        self.name.len().write(bytes);
        bytes.extend_from_slice(self.name.as_bytes());
    }

    pub(crate) fn read(bytes: &'a [u8], offset: &mut usize) -> Self {
        let parent: usize = EzBytes::read_bytes(bytes, offset);
        let str_len: usize = EzBytes::read_bytes(bytes, offset);
        let end = *offset+str_len;
        let str_bytes = &bytes[*offset..end];
        *offset = end;
        let name = std::str::from_utf8(str_bytes).unwrap(); // TODO: Error handling
        BranchId {
            name,
            parent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_formatting() {
        let root = Branch::root();
        let root_one = root.child("one");
        let root_one_a = root_one.child("a");

        fn expect(branch: &Branch, s: &str) {
            assert_eq!(format!("{:?}", branch), s);
        }

        expect(&root, "");
        expect(&root_one, ":one");
        expect(&root_one_a, ":one:a");
    }

    #[test]
    fn hash_equality() {
        let root = Branch::root();
        let root_one = root.child("one");
        let root_one_a = root_one.child("a");
        let root_one_one = root_one.child("one");

        fn eq(l: &Branch, r: &Branch) {
            assert_eq!(l.hash_128(), r.hash_128());
        }
        fn ne(l: &Branch, r: &Branch) {
            assert_ne!(l.hash_128(), r.hash_128());
        }

        ne(&root, &root_one);
        ne(&root, &root_one_one);
        ne(&root_one, &root_one_one);
        ne(&root_one_one, &root_one_a);

        eq(&root, &Branch::root());
        eq(&root_one, &Branch::root().child("one"));
        eq(&root_one_one, &Branch::root().child("one").child("one"));
    }
}
