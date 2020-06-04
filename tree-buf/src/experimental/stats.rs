use crate::prelude::*;
use std::collections::HashMap;
use std::fmt;
use std::default::Default;

#[derive(Default)]
struct Path {
    names: String,
    types: String,
}

impl Path {
    fn c(s: &String, x: &impl fmt::Display) -> String {
        let x = format!("{}", x);
        if s.is_empty() {
            x
        } else {
            if x.is_empty() {
                s.clone()
            } else {
                format!("{}.{}", s, x)
            }
        }
    }

    #[must_use]
    pub fn a(&self, name: &impl fmt::Display, type_id: &impl fmt::Display) -> Self {
        let names = Self::c(&self.names, name);
        let types = Self::c(&self.types, type_id);
        Self { names, types }
    }
}

struct PathAggregation {
    types: String,
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
            writeln!(f, "\t{}\n\t   {}\n\t   {}", agg.size, path, agg.types)?;
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

        
        let types = Path::c(&path.types, &type_id);

        let prev = self.by_path.insert(
            path.names.clone(),
            PathAggregation {
                types,
                size: len,
            },
        );
        assert!(prev.is_none());
    }
}

fn visit_array(path: Path, branch: &DynArrayBranch, breakdown: &mut SizeBreakdown) {
    match branch {
        DynArrayBranch::ArrayFixed { values, len } => visit_array(path.a(&format!("[{}]", len), &"Array Fixed"), values, breakdown),
        DynArrayBranch::Array { len, values } => {
            visit_array(path.a(&"len", &"Array"), len, breakdown);
            visit_array(path.a(&"values", &"Array"), values, breakdown);
        }
        DynArrayBranch::Enum { discriminants, variants } => {
            visit_array(path.a(&"discriminants", &"Enum"), discriminants, breakdown);
            for variant in variants.iter() {
                visit_array(path.a(&variant.ident, &"Enum"), &variant.data, breakdown);
            }
        }
        DynArrayBranch::Boolean(enc) => match enc {
            ArrayBool::Packed(b) => breakdown.add(&path, "Packed Boolean", b),
            ArrayBool::RLE(_first, runs) => visit_array(path.a(&"runs", &"Bool RLE"), runs, breakdown),
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
            visit_array(path.a(&"len", &"Map"), len, breakdown);
            visit_array(path.a(&"keys", &"Map"), keys, breakdown);
            visit_array(path.a(&"values", &"Map"), values, breakdown);
        }
        DynArrayBranch::Object { fields } => {
            for (name, field) in fields {
                visit_array(path.a(name, &"Object"), field, breakdown);
            }
        }
        DynArrayBranch::RLE { runs, values } => {
            visit_array(path.a(&"runs", &"RLE"), runs, breakdown);
            visit_array(path.a(&"values", &"RLE"), values, breakdown);
        }
        DynArrayBranch::Dictionary { indices, values } => {
            visit_array(path.a(&"indices", &"Dictionary"), indices, breakdown);
            visit_array(path.a(&"values", &"Dictionary"), values, breakdown);
        }
        DynArrayBranch::String(b) => breakdown.add(&path, "UTF-8", b),
        DynArrayBranch::Tuple { fields } => {
            for (i, field) in fields.iter().enumerate() {
                visit_array(path.a(&i, &"Tuple"), field, breakdown);
            }
        }
        DynArrayBranch::Nullable { opt, values } => {
            visit_array(path.a(&"opt", &"Nullable"), opt, breakdown);
            visit_array(path.a(&"values", &"Nullable"), values, breakdown);
        }
        DynArrayBranch::Void | DynArrayBranch::Map0 | DynArrayBranch::Array0 => {}
    }
}

fn visit(path: Path, branch: &DynRootBranch<'_>, breakdown: &mut SizeBreakdown) {
    match branch {
        DynRootBranch::Object { fields } => {
            for (name, value) in fields.iter() {
                visit(path.a(name, &"Object"), value, breakdown);
            }
        }
        DynRootBranch::Enum { discriminant, value } => visit(path.a(discriminant, &"Enum"), value, breakdown),
        DynRootBranch::Map { len: _, keys, values } => {
            visit_array(path.a(&"keys", &"Map"), keys, breakdown);
            visit_array(path.a(&"values", &"Values"), values, breakdown);
        }
        DynRootBranch::Tuple { fields } => {
            for (i, field) in fields.iter().enumerate() {
                visit(path.a(&i, &"Tuple"), field, breakdown);
            }
        }
        DynRootBranch::Map1 { key, value } => {
            visit(path.a(&"key", &"Map1"), key, breakdown);
            visit(path.a(&"value", &"Map1"), value, breakdown);
        }
        DynRootBranch::Array { len, values } => visit_array(path.a(&format!("[{}]", len), &"Array"), values, breakdown),
        DynRootBranch::Array1(item) => visit(path.a(&"1", &"Array1"), item, breakdown),
        DynRootBranch::Boolean(_)
        | DynRootBranch::Array0
        | DynRootBranch::Map0
        | DynRootBranch::Void
        | DynRootBranch::Float(_)
        | DynRootBranch::Integer(_)
        | DynRootBranch::String(_) => {}
    }
}

pub fn size_breakdown(data: &[u8]) -> DecodeResult<String> {
    let root = decode_root(data)?;

    let mut breakdown = SizeBreakdown {
        by_path: HashMap::new(),
        by_type: HashMap::new(),
        total: data.len(),
    };
    visit(Path::default(), &root, &mut breakdown);

    Ok(format!("{}", breakdown))
}
