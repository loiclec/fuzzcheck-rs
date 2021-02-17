#![allow(non_snake_case)]
use decent_synquote_alternative::{
    self as synquote,
    parser::{EnumItemData, Ty},
};

use proc_macro2::{Delimiter, Ident, Span, TokenStream};
use synquote::token_builder::*;
use synquote::{parser::TokenParser, token_builder::TokenBuilder};

mod enums;
mod tuples;

#[macro_use]
extern crate decent_synquote_alternative;

#[proc_macro]
pub fn make_basic_tuple_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut tb = TokenBuilder::new();

    let mut parser = TokenParser::new(item.into());
    if let Some(l) = parser.eat_literal() {
        if let Ok(nbr_elements) = l.to_string().parse::<usize>() {
            let fuzzcheck_mutators_crate = parser.eat_type_path().unwrap_or(ts!("fuzzcheck_mutators"));
            tuples::make_basic_tuple_mutator(&mut tb, nbr_elements, &fuzzcheck_mutators_crate);
            return tb.end().into();
        }
    }
    panic!()
}
#[proc_macro]
pub fn make_tuple_type_structure(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut tb = TokenBuilder::new();

    let mut parser = TokenParser::new(item.into());
    if let Some(l) = parser.eat_literal() {
        if let Ok(nbr_elements) = l.to_string().parse::<usize>() {
            tuples::make_tuple_type_structure(&mut tb, nbr_elements, &ts!("crate"));
            return tb.end().into();
        }
    }
    panic!()
}
#[proc_macro]
pub fn make_basic_enum_mutators(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut tb = TokenBuilder::new();

    let mut parser = TokenParser::new(item.into());
    if let Some(l) = parser.eat_literal() {
        if let Ok(nbr_elements) = l.to_string().parse::<usize>() {
            let fuzzcheck_mutators_crate = parser.eat_type_path().unwrap_or(ts!("fuzzcheck_mutators"));
            enums::make_basic_enum_mutator(&mut tb, nbr_elements, &fuzzcheck_mutators_crate);
            return tb.end().into();
        }
    }
    panic!()
}

#[proc_macro_derive(TupleStructure)]
pub fn derive_tuple_structure(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(item);
    derive_tuple_structure_(input).into()
}

#[proc_macro_attribute]
pub fn make_mutator(attribute: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let settings = MakeMutatorSettings::from(attribute.into());
    let item = proc_macro2::TokenStream::from(item);
    derive_default_mutator_(item, settings).into()
}

#[proc_macro_derive(DefaultMutator)]
pub fn derive_default_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let settings = MakeMutatorSettings::default();
    let item = proc_macro2::TokenStream::from(item);
    derive_default_mutator_(item, settings).into()
}

#[proc_macro_derive(EnumNPayloadStructure)]
pub fn derive_enum_n_payload_structure(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(item);
    derive_enum_n_payload_structure_(input).into()
}

/*
Actual implementations
*/

fn derive_tuple_structure_(item: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let input = item;
    let mut tb = TokenBuilder::new();
    let mut parser = TokenParser::new(input);

    if let Some(s) = parser.eat_struct() {
        let nbr_fields = s.struct_fields.len();
        if nbr_fields == 1 {
            tuples::impl_wrapped_tuple_1_structure(&mut tb, &s, &<_>::default());
        } else if nbr_fields > 1 {
            tuples::impl_tuple_structure_trait(&mut tb, &s, &<_>::default());
        } else {
            extend_ts!(
                &mut tb,
                "compile_error!(\"The TupleStructure macro only works for structs with one or more fields.\");"
            )
        }
    } else if let Some(_) = parser.eat_enumeration() {
        extend_ts!(
            &mut tb,
            "compile_error!(\"The TupleStructure macro cannot be used on enums.\");"
        )
    } else {
        extend_ts!(&mut tb,
            "compile_error!(\"The item could not be parsed by the TupleStructure macro. Note: only enums are supported.\");"
        )
    }
    return tb.end();
}

fn derive_enum_n_payload_structure_(item: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let input = item;
    let mut tb = TokenBuilder::new();
    let mut parser = TokenParser::new(input);

    if let Some(e) = parser.eat_enumeration() {
        if e.items
            .iter()
            .any(|item| matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if fields.len() > 0))
        {
            enums::impl_enum_structure_trait(&mut tb, &e, &ts!("fuzzcheck_mutators"));
        } else {
            extend_ts!(&mut tb,
                "compile_error!(\"The EnumNPayloadStructure macro only works on enums with at least one item with associated data.\");"
            );
        }
    } else if let Some(_) = parser.eat_struct() {
        extend_ts!(&mut tb,
            "compile_error!(\"The EnumNPayloadStructure macro only works on enums with at least one item with associated data.\");"
        );
    }
    tb.end()
}
fn derive_default_mutator_(item: proc_macro2::TokenStream, settings: MakeMutatorSettings) -> proc_macro2::TokenStream {
    let input = item;
    let mut tb = TokenBuilder::new();
    let mut parser = TokenParser::new(input);

    if let Some(s) = parser.eat_struct() {
        let nbr_fields = s.struct_fields.len();
        if nbr_fields == 0 {
            tuples::impl_default_mutator_for_struct_with_0_field(&mut tb, &s, &settings);
        } else if nbr_fields == 1 {
            tuples::impl_wrapped_tuple_1_structure(&mut tb, &s, &settings);
            tuples::impl_default_mutator_for_struct_with_1_field(&mut tb, &s, &settings);
        } else {
            tuples::impl_tuple_structure_trait(&mut tb, &s, &settings);
            tuples::impl_default_mutator_for_struct_with_more_than_1_field(&mut tb, &s, &settings);
        }
    } else if let Some(e) = parser.eat_enumeration() {
        if e.items.len() == 1 && matches!(&e.items[0].data, Some(EnumItemData::Struct(_, fields)) if fields.len() == 1)
        {
            enums::impl_wrapped_tuple_1_structure(&mut tb, &e, &settings);
            enums::impl_default_mutator_for_enum_wrapped_tuple(&mut tb, &e, &settings);
        } else if e
            .items
            .iter()
            .any(|item| matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if fields.len() > 0))
        {
            enums::impl_enum_structure_trait(&mut tb, &e, &settings.fuzzcheck_mutators_crate);
            enums::impl_default_mutator_for_enum(&mut tb, &e, &settings);
        } else if e.items.len() > 0 {
            // no associated data anywhere
            enums::impl_basic_enum_structure(&mut tb, &e, &settings);
            enums::impl_default_mutator_for_basic_enum(&mut tb, &e, &settings);
        } else {
            extend_ts!(
                &mut tb,
                "compile_error!(\"The DefaultMutator derive proc_macro does not work on empty enums.\");"
            );
        }
    } else {
        extend_ts!(&mut tb,
            "compile_error!(\"The item could not be parsed by the DefaultMutator macro. Note: only enums and structs are supported.\");"
        );
    }
    tb.end()
}

/* common */

#[derive(Debug)]
struct MakeMutatorSettings {
    name: Option<proc_macro2::Ident>,
    recursive: bool,
    default: bool,
    fuzzcheck_mutators_crate: proc_macro2::TokenStream,
}
impl MakeMutatorSettings {
    // TODO: don't panic like that, add a nice compile error
    fn from(attribute: proc_macro2::TokenStream) -> Self {
        let mut parser = TokenParser::new(attribute);
        let mut name = None;
        let mut recursive = None;
        let mut default = None;
        let mut fuzzcheck_mutators_crate = None;
        while !parser.is_eot() {
            if let Some(ident) = parser.eat_any_ident() {
                match ident.to_string().as_ref() {
                    "name" => {
                        if parser.eat_punct(':').is_none() {
                            panic!()
                        }
                        if let Some(ident) = parser.eat_any_ident() {
                            name = Some(ident);
                        } else {
                            panic!()
                        }
                    }
                    "recursive" => {
                        if parser.eat_punct(':').is_none() {
                            panic!()
                        }
                        if parser.eat_ident("true").is_some() {
                            recursive = Some(true);
                        } else if parser.eat_ident("false").is_some() {
                            recursive = Some(false);
                        } else {
                            panic!()
                        }
                    }
                    "default" => {
                        if parser.eat_punct(':').is_none() {
                            panic!()
                        }
                        if parser.eat_ident("true").is_some() {
                            default = Some(true);
                        } else if parser.eat_ident("false").is_some() {
                            default = Some(false);
                        } else {
                            panic!()
                        }
                    }
                    "fuzzcheck_mutators_crate" => {
                        if parser.eat_punct(':').is_none() {
                            panic!()
                        }
                        if let Some(ts) = parser.eat_type_path() {
                            fuzzcheck_mutators_crate = Some(ts);
                        } else {
                            panic!()
                        }
                    }
                    _ => {
                        panic!()
                    }
                }
                let _ = parser.eat_punct(',');
            } else {
                panic!()
            }
        }
        let default_settings = MakeMutatorSettings::default();
        MakeMutatorSettings {
            name,
            recursive: recursive.unwrap_or(default_settings.recursive),
            default: default.unwrap_or(default_settings.default),
            fuzzcheck_mutators_crate: fuzzcheck_mutators_crate.unwrap_or(default_settings.fuzzcheck_mutators_crate),
        }
    }
}
impl Default for MakeMutatorSettings {
    fn default() -> Self {
        MakeMutatorSettings {
            name: None,
            recursive: false,
            default: true,
            fuzzcheck_mutators_crate: ts!("fuzzcheck_mutators"),
        }
    }
}

#[allow(non_snake_case)]
pub(crate) struct Common {
    //fuzzcheck_mutators_crate: TokenStream,
    // EitherNP1_pathTi: Box<dyn (Fn(usize) -> TokenStream)>,
    // EnumNPayloadArbitraryStep_path: TokenStream,
    // EnumNPayloadMutationStep_path: TokenStream,
    // WrappedTupleN_ident: Ident,
    Clone: TokenStream,
    Default: TokenStream,
    DefaultMutator: TokenStream,
    EitherNP1_ident: Ident,
    EitherNP1_identTi: Box<dyn (Fn(usize) -> TokenStream)>,
    EitherNP1_path: TokenStream,
    EnumNPayloadArbitraryStep_ident: Ident,
    EnumNPayloadMutationStep_ident: Ident,
    EnumNPayloadMutator_ident: Ident,
    EnumNPayloadMutator_path: TokenStream,
    EnumNPayloadStructure_ident: Ident,
    EnumNPayloadStructure_path: TokenStream,
    fastrand: TokenStream,
    fastrand_Rng: TokenStream,
    fuzzcheck_mutator_traits_Mutator: TokenStream,
    fuzzcheck_traits_Mutator: TokenStream,
    Mi_j: Box<dyn (Fn(usize, usize) -> Ident)>,
    Mi: Box<dyn (Fn(usize) -> Ident)>,
    mutator_i: Box<dyn (Fn(usize) -> Ident)>,
    None: TokenStream,
    Option: TokenStream,
    PhantomData: TokenStream,
    RefTypes: TokenStream,
    size_to_cplxity: TokenStream,
    Some: TokenStream,
    ti: Box<dyn (Fn(usize) -> Ident)>,
    ti_value: Box<dyn (Fn(usize) -> Ident)>,
    ti_cache: Box<dyn (Fn(usize) -> Ident)>,
    Ti: Box<dyn (Fn(usize) -> Ident)>,
    Tuplei: Box<dyn (Fn(usize) -> TokenStream)>,
    TupleKindi: Box<dyn (Fn(usize) -> Ident)>,
    TupleMutator: TokenStream,
    TupleMutatorTiTupleKindi: Box<dyn (Fn(usize) -> TokenStream)>,
    TupleN_ident: Ident,
    TupleN_path: TokenStream,
    TupleNMutator: Box<dyn Fn(usize) -> TokenStream>,
    TupleNMutator_ident: Ident,
    TupleStructure: TokenStream,
    TupleStructureTupleKindi: Box<dyn (Fn(usize) -> TokenStream)>,
    variant_count_T: TokenStream,
    UnitMutator: TokenStream,
    Vec: TokenStream,
    WrappedTupleN_path: TokenStream,
    WrappedMutator_path: TokenStream,
}
impl Common {
    #[allow(non_snake_case)]
    fn new(fuzzcheck_mutators_crate: &TokenStream, n: usize) -> Self {
        let ti = Box::new(|i: usize| ident!("t" i));
        let ti_value = Box::new(|i: usize| ident!("t" i "_value"));
        let ti_cache = Box::new(|i: usize| ident!("t" i "_cache"));
        let Ti = Box::new(|i: usize| ident!("T" i));
        let TupleKindi = Box::new(|i: usize| ident!("TupleKind" i));
        let TupleMutator = ts!(fuzzcheck_mutators_crate "::TupleMutator");
        let TupleMutatorTiTupleKindi = {
            let Ti = Ti.clone();
            let TupleKindi = TupleKindi.clone();
            let TupleMutator = TupleMutator.clone();
            Box::new(move |i: usize| ts!(TupleMutator "<" Ti(i) "," TupleKindi(i) ">"))
        };
        let Option = ts!("::std::option::Option");
        let some = ts!(Option "::Some");
        let none = ts!(Option "::None");
        let EnumNPayloadArbitraryStep_ident = ident!("Enum" n "PayloadArbitraryStep");
        // let EnumNPayloadArbitraryStep_path = ts!(fuzzcheck_mutators_crate "::" EnumNPayloadArbitraryStep_ident);
        let EnumNPayloadMutationStep_ident = ident!("Enum" n "PayloadMutationStep");
        // let EnumNPayloadMutationStep_path = ts!(fuzzcheck_mutators_crate "::" EnumNPayloadMutationStep_ident);
        let EitherNP1_ident = ident!("Either" n+1);
        let EitherNP1_identTi = {
            let Either = EitherNP1_ident.clone();
            let Ti = Ti.clone();
            Box::new(move |i: usize| ts!(Either "::" Ti(i)))
        };
        let EitherNP1_path = ts!(fuzzcheck_mutators_crate "::" ident!("Either" n+1));
        // let EitherNP1_pathTi = {
        //     let Either = EitherNP1_path.clone();
        //     let Ti = Ti.clone();
        //     Box::new(move |i: usize| ts!(Either "::" Ti(i)))
        // };
        let TupleStructure = ts!(fuzzcheck_mutators_crate "::TupleStructure");
        let TupleStructureTupleKindi = {
            let TupleStructure = TupleStructure.clone();
            let TupleKindi = TupleKindi.clone();
            Box::new(move |i: usize| ts!(TupleStructure "<" TupleKindi(i) ">"))
        };
        let Tuplei = {
            let fuzzcheck_mutators_crate = fuzzcheck_mutators_crate.clone();
            Box::new(move |i: usize| ts!(fuzzcheck_mutators_crate "::" ident!("Tuple" i)))
        };
        let TupleNMutator = {
            let fuzzcheck_mutators_crate = fuzzcheck_mutators_crate.clone();
            Box::new(move |n: usize| ts!(fuzzcheck_mutators_crate "::" ident!("Tuple" n "Mutator")))
        };
        let EnumNPayloadStructure_ident = ident!("Enum" n "PayloadStructure");
        let EnumNPayloadStructure_path = ts!(fuzzcheck_mutators_crate "::" EnumNPayloadStructure_ident);
        let EnumNPayloadMutator_ident = ident!("Enum" n "PayloadMutator");
        let EnumNPayloadMutator_path = ts!(fuzzcheck_mutators_crate "::" EnumNPayloadMutator_ident);

        let fuzzcheck_traits_Mutator = ts!("::fuzzcheck_traits::Mutator");

        let WrappedTupleN_ident = ident!("WrappedTuple" n);
        let WrappedTupleN_path = ts!(fuzzcheck_mutators_crate "::" WrappedTupleN_ident);
        let WrappedMutator_path = ts!(fuzzcheck_mutators_crate "::WrappedMutator");

        let fastrand = ts!(fuzzcheck_mutators_crate "::fastrand");
        let fastrand_Rng = ts!(fastrand "::Rng");

        Self {
            // EitherNP1_pathTi,
            // EnumNPayloadArbitraryStep_path,
            // EnumNPayloadMutationStep_path,
            // fuzzcheck_mutators_crate: fuzzcheck_mutators_crate.clone(),
            // WrappedTupleN_ident,
            Clone: ts!("::std::clone::Clone"),
            Default: ts!("::std::default::Default"),
            DefaultMutator: ts!(fuzzcheck_mutators_crate "::DefaultMutator"),
            EitherNP1_ident,
            EitherNP1_identTi,
            EitherNP1_path,
            EnumNPayloadArbitraryStep_ident,
            EnumNPayloadMutationStep_ident,
            EnumNPayloadMutator_ident,
            EnumNPayloadMutator_path,
            EnumNPayloadStructure_ident,
            EnumNPayloadStructure_path,
            fastrand,
            fastrand_Rng,
            fuzzcheck_mutator_traits_Mutator: ts!(fuzzcheck_mutators_crate fuzzcheck_traits_Mutator),
            fuzzcheck_traits_Mutator,
            Mi_j: Box::new(|i, j| ident!("M" i "_" j)),
            Mi: Box::new(|i| ident!("M" i)),
            mutator_i: Box::new(|i: usize| ident!("mutator_" i)),
            None: none,
            Option,
            PhantomData: ts!("::std::marker::PhantomData"),
            RefTypes: ts!(fuzzcheck_mutators_crate "::RefTypes"),
            size_to_cplxity: ts!(fuzzcheck_mutators_crate "::size_to_cplxity"),
            Some: some,
            ti,
            ti_value,
            ti_cache,
            Ti,
            Tuplei,
            TupleKindi,
            TupleMutator,
            TupleMutatorTiTupleKindi,
            TupleN_ident: ident!("Tuple" n),
            TupleN_path: ts!(fuzzcheck_mutators_crate "::" ident!("Tuple" n)),
            TupleNMutator,
            TupleNMutator_ident: ident!("Tuple" n "Mutator"),
            TupleStructure,
            TupleStructureTupleKindi,
            UnitMutator: ts!(fuzzcheck_mutators_crate "::UnitMutator"),
            variant_count_T: ts!("::std::mem::variant_count::<T>()"),
            Vec: ts!("::std::vec::Vec"),
            WrappedTupleN_path,
            WrappedMutator_path,
        }
    }
}

fn read_field_default_mutator_attribute(attribute: TokenStream) -> Option<Ty> {
    let mut parser = TokenParser::new(attribute);
    let _ = parser.eat_punct('#');
    let content = match parser.eat_group(Delimiter::Bracket) {
        Some(proc_macro2::TokenTree::Group(group)) => group,
        Some(_) => panic!(),
        None => return None,
    };
    let mut parser = TokenParser::new(content.stream());
    let _ = parser.eat_ident("field_mutator")?;
    let content = match parser.eat_any_group() {
        Some(proc_macro2::TokenTree::Group(group)) => group,
        Some(_) => panic!(),
        None => return None,
    };
    let mut parser = TokenParser::new(content.stream());
    let ty = parser.eat_type();
    // eprintln!("{:?}", ts!(ty));
    ty
}
