use crate::prelude::*;

pub trait Visitor {
    fn visit_root(&mut self, root: &DynRootBranch<'_>) {}
    fn visit_array(&mut self, array: &DynArrayBranch<'_>) {}
}

pub fn visit_all(root: &DynRootBranch<'_>, visitor: &mut impl Visitor) {
    profile_fn!(visit_all);
    let mut roots = vec![root];
    let mut arrays = Vec::new();
    while !roots.is_empty() || !arrays.is_empty() {
        while let Some(root) = roots.pop() {
            visitor.visit_root(root);
            use DynRootBranch::*;
            match root {
                Object { fields } => {
                    for field in fields.values() {
                        roots.push(child);
                    }
                }
                Array0 => {}
                Array1(inner) => {
                    roots.push(inner);
                }
                Array { len: _, values } => {
                    arrays.push(values);
                }
                _ => todo!("visitor"),
            }
        }
    }
}
