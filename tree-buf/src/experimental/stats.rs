use crate::prelude::*;
use std::collections::HashMap;
use std::fmt;

struct Path {
    value: String,
}

impl Path {
    pub fn root() -> Self {
        Self { value: String::new() }
    }

    #[must_use]
    pub fn a(&self, p: &impl fmt::Display) -> Self {
        let p = format!("{}", p);
        let value = if self.value.is_empty() {
            p
        } else {
            if p.is_empty() {
                self.value.clone()
            } else {
                format!("{}.{}", self.value, p)
            }
        };
        Self { value }
    }
}

struct PathAggregation {
    type_id: String,
    size: usize,
}

#[derive(Default, Clone)]
struct TypeAggregation {
    size: usize,
    count: usize,
}

struct SizeBreakdown {
    by_path: HashMap<String, PathAggregation>,
    by_type: HashMap<String, TypeAggregation>,
    total: usize,
}

impl fmt::Display for SizeBreakdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut by_path: Vec<_> = self.by_path.iter().collect();
        let mut by_type: Vec<_> = self.by_type.iter().collect();

        by_path.sort_by_key(|i| usize::MAX - i.1.size);
        by_type.sort_by_key(|i| usize::MAX - i.1.size);

        writeln!(f, "Largest by path:")?;
        for (path, agg) in by_path.iter() {
            writeln!(f, "\t{} {} {}", agg.size, agg.type_id, path)?;
        }

        writeln!(f)?;
        writeln!(f, "Largest by type:")?;
        for (t, agg) in by_type.iter() {
            writeln!(f, "\t {}x {} @ {}", agg.count, agg.size, t)?;
        }

        let accounted: usize = by_type.iter().map(|i| (i.1).size).sum();

        writeln!(f)?;
        writeln!(f, "Other: {}", self.total - accounted)?;
        writeln!(f, "Total: {}", self.total)?;

        Ok(())
    }
}

impl SizeBreakdown {
    fn add(&mut self, path: &Path, type_id: &'static str, bytes: &Bytes<'_>) {
        let len = bytes.len();
        let before = self.by_type.get(type_id).cloned().unwrap_or_default();
        self.by_type.insert(
            type_id.to_owned(),
            TypeAggregation {
                count: before.count + 1,
                size: before.size + len,
            },
        );

        let prev = self.by_path.insert(
            path.value.to_owned(),
            PathAggregation {
                type_id: type_id.to_owned(),
                size: len,
            },
        );
        assert!(prev.is_none());
    }
}

fn visit_array(path: &Path, append: &impl fmt::Display, branch: &DynArrayBranch, breakdown: &mut SizeBreakdown) {
    let path = path.a(append);
    match branch {
        DynArrayBranch::ArrayFixed { values, len: _ } => visit_array(&path, &"", values, breakdown),
        DynArrayBranch::Array { len, values } => {
            visit_array(&path, &"len", len, breakdown);
            visit_array(&path, &"values", values, breakdown);
        }
        DynArrayBranch::Enum { discriminants, variants } => {
            visit_array(&path, &"", discriminants, breakdown);
            for variant in variants.iter() {
                visit_array(&path, &variant.ident, &variant.data, breakdown);
            }
        }
        DynArrayBranch::Boolean(enc) => match enc {
            ArrayBool::Packed(b) => breakdown.add(&path, "Packed Boolean", b),
            ArrayBool::RLE(_first, runs) => visit_array(&path, &"runs", runs, breakdown),
        },
        DynArrayBranch::Float(f) => match f {
            ArrayFloat::DoubleGorilla(b) => breakdown.add(&path, "Gorilla", b),
            ArrayFloat::F32(b) => breakdown.add(&path, "Fixed F32", b),
            ArrayFloat::F64(b) => breakdown.add(&path, "Fixed F64", b),
            ArrayFloat::Zfp32(b) => breakdown.add(&path, "Zfp 64", b),
            ArrayFloat::Zfp64(b) => breakdown.add(&path, "Zfp 32", b),
        },
        DynArrayBranch::Integer(ArrayInteger { bytes, encoding }) => match encoding {
            ArrayIntegerEncoding::PrefixVarInt => breakdown.add(&path, "Prefix Varint", bytes),
            ArrayIntegerEncoding::Simple16 => breakdown.add(&path, "Simple16", bytes),
            ArrayIntegerEncoding::U8 => breakdown.add(&path, "U8 Fixed", bytes),
        },
        DynArrayBranch::Map { len, keys, values } => {
            visit_array(&path, &"len", len, breakdown);
            visit_array(&path, &"keys", keys, breakdown);
            visit_array(&path, &"values", values, breakdown);
        }
        DynArrayBranch::Object { fields } => {
            for (name, field) in fields {
                visit_array(&path, name, field, breakdown);
            }
        }
        DynArrayBranch::RLE { runs, values } => {
            visit_array(&path, &"runs", runs, breakdown);
            visit_array(&path, &"values", values, breakdown);
        }
        DynArrayBranch::Dictionary { indices, values } => {
            visit_array(&path, &"indices", indices, breakdown);
            visit_array(&path, &"values", values, breakdown);
        }
        DynArrayBranch::String(b) => breakdown.add(&path, "UTF-8", b),
        DynArrayBranch::Tuple { fields } => {
            for (i, field) in fields.iter().enumerate() {
                visit_array(&path, &i, field, breakdown);
            }
        }
        DynArrayBranch::Nullable { opt, values } => {
            visit_array(&path, &"opt", opt, breakdown);
            visit_array(&path, &"values", values, breakdown);
        }
        DynArrayBranch::Void | DynArrayBranch::Map0 | DynArrayBranch::Array0 => {}
    }
}

fn visit(path: &Path, append: impl fmt::Display, branch: &DynRootBranch<'_>, breakdown: &mut SizeBreakdown) {
    let path = path.a(&append);
    match branch {
        DynRootBranch::Object { fields } => {
            for (name, value) in fields.iter() {
                visit(&path, name, value, breakdown);
            }
        }
        DynRootBranch::Enum { discriminant, value } => visit(&path, discriminant, value, breakdown),
        DynRootBranch::Map { len: _, keys, values } => {
            visit_array(&path, &"keys", keys, breakdown);
            visit_array(&path, &"values", values, breakdown);
        }
        DynRootBranch::Tuple { fields } => {
            for (i, field) in fields.iter().enumerate() {
                visit(&path, i, field, breakdown);
            }
        }
        DynRootBranch::Map1 { key, value } => {
            visit(&path, "key", key, breakdown);
            visit(&path, "value", value, breakdown);
        }
        DynRootBranch::Array { len: _, values } => visit_array(&path, &"", values, breakdown),
        DynRootBranch::Array1(item) => visit(&path, "", item, breakdown),
        DynRootBranch::Boolean(_)
        | DynRootBranch::Array0
        | DynRootBranch::Map0
        | DynRootBranch::Void
        | DynRootBranch::Float(_)
        | DynRootBranch::Integer(_)
        | DynRootBranch::String(_) => {}
    }
}

pub fn size_breakdown(data: &[u8]) -> ReadResult<String> {
    let root = read_root(data)?;

    let mut breakdown = SizeBreakdown {
        by_path: HashMap::new(),
        by_type: HashMap::new(),
        total: data.len(),
    };
    visit(&Path::root(), "", &root, &mut breakdown);

    Ok(format!("{}", breakdown))
}
