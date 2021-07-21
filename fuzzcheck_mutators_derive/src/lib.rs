#![allow(non_snake_case)]

use decent_synquote_alternative::{
    self as synquote,
    parser::{EnumItemData, Ty},
};

use proc_macro2::{Delimiter, Ident, Span, TokenStream};
use synquote::token_builder::*;
use synquote::{parser::TokenParser, token_builder::TokenBuilder};

mod enums;
mod single_variant;
mod structs_and_enums;
mod tuples;

#[macro_use]
extern crate decent_synquote_alternative;

#[proc_macro]
pub fn make_basic_tuple_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut tb = TokenBuilder::new();

    let mut parser = TokenParser::new(item.into());
    if let Some(l) = parser.eat_literal() {
        if let Ok(nbr_elements) = l.to_string().parse::<usize>() {
            tuples::make_basic_tuple_mutator(&mut tb, nbr_elements);
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
            tuples::make_tuple_type_structure(&mut tb, nbr_elements);
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

#[proc_macro]
pub fn make_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (settings, parser) = MakeMutatorSettings::from(item.clone().into());
    // let item = proc_macro2::TokenStream::from(item);
    derive_default_mutator_(parser, settings).into()
}

#[proc_macro_derive(DefaultMutator)]
pub fn derive_default_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let settings = MakeMutatorSettings::default();
    let item = proc_macro2::TokenStream::from(item);
    let parser = TokenParser::new(item);
    derive_default_mutator_(parser, settings).into()
}

#[proc_macro]
pub fn make_single_variant_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(item);
    make_single_variant_mutator_(input).into()
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
        if nbr_fields > 0 {
            tuples::impl_tuple_structure_trait(&mut tb, &s);
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

fn derive_default_mutator_(mut parser: TokenParser, settings: MakeMutatorSettings) -> proc_macro2::TokenStream {
    let mut tb = TokenBuilder::new();

    if let Some(s) = parser.eat_struct() {
        let nbr_fields = s.struct_fields.len();
        if nbr_fields == 0 {
            tuples::impl_default_mutator_for_struct_with_0_field(&mut tb, &s);
        } else {
            tuples::impl_tuple_structure_trait(&mut tb, &s);
            tuples::impl_default_mutator_for_struct(&mut tb, &s, &settings);
        }
    } else if let Some(e) = parser.eat_enumeration() {
        if e.items
            .iter()
            .any(|item| matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if fields.len() > 0))
        {
            single_variant::make_single_variant_mutator(&mut tb, &e);
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

fn make_single_variant_mutator_(item: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let input = item;
    let mut tb = TokenBuilder::new();
    let mut parser = TokenParser::new(input);

    if let Some(e) = parser.eat_enumeration() {
        single_variant::make_single_variant_mutator(&mut tb, &e);
    } else {
        extend_ts!(&mut tb,
            "compile_error!(\"The item could not be parsed by the make_single_variant_mutator macro. Note: only enums are supported.\");"
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
    fn from(attribute: proc_macro2::TokenStream) -> (MakeMutatorSettings, TokenParser) {
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
                    "type" => {
                        if parser.eat_punct(':').is_none() {
                            panic!()
                        }
                        let default_settings = MakeMutatorSettings::default();
                        return (
                            MakeMutatorSettings {
                                name,
                                recursive: recursive.unwrap_or(default_settings.recursive),
                                default: default.unwrap_or(default_settings.default),
                                fuzzcheck_mutators_crate: fuzzcheck_mutators_crate
                                    .unwrap_or(default_settings.fuzzcheck_mutators_crate),
                            },
                            parser,
                        );
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
        panic!()
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
    AlternationMutator: TokenStream,
    Clone: TokenStream,
    Default: TokenStream,
    DefaultMutator: TokenStream,
    fastrand_Rng: TokenStream,
    // fuzzcheck_mutator_traits_Mutator: TokenStream,
    fuzzcheck_traits_Mutator: TokenStream,
    Mi_j: Box<dyn (Fn(usize, usize) -> Ident)>,
    Mi: Box<dyn (Fn(usize) -> Ident)>,
    mutator_i: Box<dyn (Fn(usize) -> Ident)>,
    None: TokenStream,
    Option: TokenStream,
    PhantomData: TokenStream,
    RefTypes: TokenStream,
    Some: TokenStream,
    ti: Box<dyn (Fn(usize) -> Ident)>,
    ti_value: Box<dyn (Fn(usize) -> Ident)>,
    Ti: Box<dyn (Fn(usize) -> Ident)>,
    Tuplei: Box<dyn (Fn(usize) -> TokenStream)>,
    TupleMutator: TokenStream,
    TupleMutatorWrapper: TokenStream,
    TupleN_ident: Ident,
    TupleN_path: TokenStream,
    TupleNMutator: Box<dyn Fn(usize) -> TokenStream>,
    TupleNMutator_ident: Ident,
    TupleStructure: TokenStream,
    UnitMutator: TokenStream,
    Vec: TokenStream,
    VoseAlias: TokenStream,
    RecursiveMutator: TokenStream,
    Box: TokenStream,
}
impl Common {
    #[allow(non_snake_case)]
    fn new(n: usize) -> Self {
        let fuzzcheck_mutators_crate = ts!("fuzzcheck_mutators");
        let ti = Box::new(|i: usize| ident!("t" i));
        let ti_value = Box::new(|i: usize| ident!("t" i "_value"));
        let Ti = Box::new(|i: usize| ident!("T" i));
        let TupleMutator = ts!(fuzzcheck_mutators_crate "::tuples::TupleMutator");
        let Option = ts!("::std::option::Option");
        let some = ts!(Option "::Some");
        let none = ts!(Option "::None");

        let TupleStructure = ts!(fuzzcheck_mutators_crate "::tuples::TupleStructure");
        let Tuplei = {
            let fuzzcheck_mutators_crate = fuzzcheck_mutators_crate.clone();
            Box::new(move |i: usize| ts!(fuzzcheck_mutators_crate "::tuples::" ident!("Tuple" i)))
        };
        let TupleNMutator = {
            let fuzzcheck_mutators_crate = fuzzcheck_mutators_crate.clone();
            Box::new(move |n: usize| ts!(fuzzcheck_mutators_crate "::tuples::" ident!("Tuple" n "Mutator")))
        };

        let fuzzcheck_traits_Mutator = ts!("fuzzcheck_mutators::fuzzcheck_traits::Mutator");

        let fastrand = ts!(fuzzcheck_mutators_crate "::fastrand");
        let fastrand_Rng = ts!(fastrand "::Rng");

        Self {
            AlternationMutator: ts!(fuzzcheck_mutators_crate "::alternation::AlternationMutator"),
            Clone: ts!("::std::clone::Clone"),
            Default: ts!("::std::default::Default"),
            DefaultMutator: ts!(fuzzcheck_mutators_crate "::DefaultMutator"),
            fastrand_Rng,
            // fuzzcheck_mutator_traits_Mutator: ts!(fuzzcheck_mutators_crate fuzzcheck_traits_Mutator),
            fuzzcheck_traits_Mutator,
            Mi_j: Box::new(|i, j| ident!("M" i "_" j)),
            Mi: Box::new(|i| ident!("M" i)),
            mutator_i: Box::new(|i: usize| ident!("mutator_" i)),
            None: none,
            Option,
            PhantomData: ts!("::std::marker::PhantomData"),
            RefTypes: ts!(fuzzcheck_mutators_crate "::tuples::RefTypes"),
            Some: some,
            ti,
            ti_value,
            Ti,
            Tuplei,
            TupleMutator,
            TupleMutatorWrapper: ts!(fuzzcheck_mutators_crate "::tuples::TupleMutatorWrapper"),
            TupleN_ident: ident!("Tuple" n),
            TupleN_path: ts!(fuzzcheck_mutators_crate "::tuples::" ident!("Tuple" n)),
            TupleNMutator,
            TupleNMutator_ident: ident!("Tuple" n "Mutator"),
            TupleStructure,
            UnitMutator: ts!(fuzzcheck_mutators_crate "::unit::UnitMutator"),
            Vec: ts!("::std::vec::Vec"),
            VoseAlias: ts!(fuzzcheck_mutators_crate "::vose_alias::VoseAlias"),
            RecursiveMutator: ts!(fuzzcheck_mutators_crate "::recursive::RecursiveMutator"),
            Box: ts!("::std::boxed::Box"),
        }
    }
}

fn read_field_default_mutator_attribute(attribute: TokenStream) -> Option<(Ty, Option<TokenStream>)> {
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
    if let Some(ty) = ty {
        if parser.eat_punct('=').is_some() {
            if let Some(init) = parser.eat_group(Delimiter::Brace) {
                match init {
                    proc_macro2::TokenTree::Group(g) => Some((ty, Some(g.stream()))),
                    _ => unreachable!(),
                }
            } else {
                panic!()
            }
        } else {
            Some((ty, None))
        }
    } else {
        None
    }
    // eprintln!("{:?}", ts!(ty));
}
