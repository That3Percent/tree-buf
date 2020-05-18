use {
    inflector::cases::camelcase::to_camel_case,
    proc_macro2::Ident,
    syn::{DataStruct, Fields, Type},
};

// TODO: If we limit ourselves to ASCII characters, then it's possible to just do the same as prefix-varint and have a tag bit to save binary size
// TODO: Semantically this is a sequence of case-folded canonically encoded utf-8 words (though, this is not quite implemented as such here)
// This is prime for some kind of dictionary compression. Most applications won't ever need to ship the dictionary since it only
// would happen in the proc-macro, except when introspection is required. (Reader for example just compares byte arrays)
// or compression, and that can just happen in the proc-macro.
// TODO: Ensure that leading separators are preserved?
// TODO: Unfortunately, the current method is quite inadequate. Consider a language with no case. Consider a letter 'q' having
// neither uppercase nor lowercase. qq vs q_q is different. But, in this encoding they are the same.
pub fn canonical_ident(ident: &Ident) -> String {
    let ident_str = format!("{}", ident);
    to_camel_case(&ident_str)
}

pub struct NamedField<'a> {
    pub ident: &'a Ident,
    pub ty: &'a Type,
    pub canon_str: String,
}
pub type NamedFields<'a> = Vec<NamedField<'a>>;

pub fn get_named_fields(data_struct: &DataStruct) -> NamedFields {
    // TODO: Lift restriction
    let fields_named = match &data_struct.fields {
        Fields::Named(fields_named) => fields_named,
        _ => panic!("The struct must have named fields"),
    };

    fields_named
        .named
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            NamedField {
                ident: field.ident.as_ref().unwrap(),
                ty: &field.ty,
                canon_str: canonical_ident(&ident),
            }
        })
        .collect()
}
