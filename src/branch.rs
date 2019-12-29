use std::fmt::{Debug, Formatter, Error, Write};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::{DefaultHasher, HashMap};

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

struct FlatBranch<'a, D> {
    index: usize,
    parent: Option<usize>,
    name: &'a str,
    data: D, // Either a reader or a writer.
}

pub(crate) struct BranchMap<'a, D> {
    // TODO: Ultimately the 128 bit hash shouldn't be necessary, instead some kind of FlatBranch can be used all the time, with &'static str for writer and a string tied to the buffer for reader.
    sticks: HashMap<u128, FlatBranch<'a, D>>,
}

impl<'a, D> BranchMap<'a, D> {
    pub fn new() -> Self {
        Self {
            sticks: HashMap::new(),
        }
    }
}

impl <'a, D: Default> BranchMap<'a, D> {
    fn get_or_default_mut(&mut self, key: &'a Branch) -> &mut FlatBranch<'a, D> {
        let hash = key.hash_128();
        todo!();
        /*
        if let Some(result) = self.sticks.get_mut(&hash) {
            return result;
        }
        let parent = key.prev.map(|parent| self.get_or_default_mut(parent).index);
        let flat = FlatBranch {
            index: self.sticks.len(),
            parent,
            name: key.namespace,
            data: D::default(),
        };

        self.sticks.insert(hash, flat);
        self.sticks.get_mut(&hash).unwrap()
        */
    }
    pub fn get_data_mut(&mut self, key: &'a Branch) -> &mut D {
        &mut self.get_or_default_mut(key).data
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

    #[test]
    fn data() {
        /*
        let mut branch_map = BranchMap::new();

        let root = Branch::root();
        let root_one = root.child("one");

        assert_eq!(&0u32, branch_map.get_data_mut(&root));

        *branch_map.get_data_mut(&root_one) += 2;
        *branch_map.get_data_mut(&root_one) += 2;
        assert_eq!(&0u32, branch_map.get_data_mut(&root_one));
        */
        
    }
}
