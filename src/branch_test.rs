use std::fmt::{Debug, Formatter, Error, Write};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::{DefaultHasher, HashMap};


// TODO: In file, store branch as -
// Prev branch (id, # in file), name, type, data ptr

#[derive(Clone, Hash, Eq, PartialEq)]
struct Branch<'a> {
    namespace: &'static str,
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
    pub fn root() -> Self {
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

struct FlatBranch<'a, D> {
    index: usize,
    parent: Option<usize>,
    name: &'a str,
    data: D, // Either a reader or a writer.
}

struct BranchMap<'a, D> {
    sticks: HashMap<u128, FlatBranch<'a, D>>,
}

#[test]
fn branch_test() {
    let root = Branch::root();
    let one = root.child("one");
    let two = root.child("two");
    let one_a = one.child("a");

    assert_ne!(one.hash_128(), two.hash_128());

    assert_eq!(format!("{:?}", one_a), ":one:a");
}