#![allow(non_snake_case)]
#![allow(clippy::type_complexity)]

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

/// Create a tuple-mutatpr for of the given arity.
///
/// This function can only be used within fuzzcheck itself.
///
/// ```ignore
/// make_basic_tuple_mutator!(2);
/// // now the type Tuple2Mutator<M1, M2> is available
/// // It implements TupleMutator<T, Tuple2<A, B>> if M1: Mutator<A> and M2: Mutator<B>
/// let tuple_mutator = Tuple2Mutator::new(bool::default_mutator(), i8::default_mutator());
/// // tuple_mutator impl Tuple2Mutator<T, Tuple2<bool, i8>>
/// // to get a regular Mutator<(A, B)>, wrap the generated tuple-mutator with a TupleMutatorWrapper
/// let mutator = TupleMutatorWrapper::new(tuple_mutator);
/// // mutator impl Mutator<(A, B)>
/// ```
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

#[doc(hidden)]
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

#[doc(hidden)]
#[proc_macro_derive(TupleStructure)]
pub fn derive_tuple_structure(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(item);
    derive_tuple_structure_(input).into()
}
#[proc_macro]
pub fn make_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (settings, parser) = MakeMutatorSettings::from(item.into());
    // let item = proc_macro2::TokenStream::from(item);
    derive_default_mutator_(parser, settings).into()
}

#[proc_macro_derive(DefaultMutator, attributes(field_mutator))]
pub fn derive_default_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let settings = MakeMutatorSettings::default();
    let item = proc_macro2::TokenStream::from(item);
    let parser = TokenParser::new(item);
    derive_default_mutator_(parser, settings).into()
}

#[doc(hidden)]
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
    } else if parser.eat_enumeration().is_some() {
        extend_ts!(
            &mut tb,
            "compile_error!(\"The TupleStructure macro cannot be used on enums.\");"
        )
    } else {
        extend_ts!(&mut tb,
            "compile_error!(\"The item could not be parsed by the TupleStructure macro. Note: only enums are supported.\");"
        )
    }
    tb.end()
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
            .any(|item| matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if !fields.is_empty()))
        {
            single_variant::make_single_variant_mutator(&mut tb, &e);
            enums::impl_default_mutator_for_enum(&mut tb, &e, &settings);
        } else if !e.items.is_empty() {
            // no associated data anywhere
            enums::impl_basic_enum_structure(&mut tb, &e);
            enums::impl_default_mutator_for_basic_enum(&mut tb, &e);
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
}
impl MakeMutatorSettings {
    // TODO: don't panic like that, add a nice compile error
    fn from(attribute: proc_macro2::TokenStream) -> (MakeMutatorSettings, TokenParser) {
        let mut parser = TokenParser::new(attribute);
        let mut name = None;
        let mut recursive = None;
        let mut default = None;
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
        }
    }
}

#[allow(non_snake_case)]
/// Stores common syntax that is useful throughout the procedural macro. This struct contains
/// precise and unambiguous references to items within the main fuzzcheck library.
///
/// Many functions in this struct of type `usize -> TokenStream` generate `TokenStream`s in the form
/// `{some constant}{usize}`.
pub(crate) struct Common {
    AlternationMutator: TokenStream,
    Clone: TokenStream,
    Default: TokenStream,
    DefaultMutator: TokenStream,
    fastrand_Rng: TokenStream,
    mutators: TokenStream,
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
    /// Generates identifiers of the form `Tuple{n}Mutator`.
    TupleNMutator: Box<dyn Fn(usize) -> TokenStream>,
    TupleNMutator_ident: Ident,
    TupleStructure: TokenStream,
    UnitMutator: TokenStream,
    Vec: TokenStream,
    VoseAlias: TokenStream,
    RecursiveMutator: TokenStream,
    Box: TokenStream,
    NeverMutator: TokenStream,
}
impl Common {
    #[allow(non_snake_case)]
    fn new(n: usize) -> Self {
        let mutators = ts!("fuzzcheck::mutators");
        let ti = Box::new(|i: usize| ident!("t" i));
        let ti_value = Box::new(|i: usize| ident!("t" i "_value"));
        let Ti = Box::new(|i: usize| ident!("T" i));
        let TupleMutator = ts!(mutators "::tuples::TupleMutator");
        let Option = ts!("::std::option::Option");
        let some = ts!(Option "::Some");
        let none = ts!(Option "::None");

        let TupleStructure = ts!(mutators "::tuples::TupleStructure");
        let Tuplei = {
            let mutators = mutators.clone();
            Box::new(move |i: usize| ts!(mutators "::tuples::" ident!("Tuple" i)))
        };
        let TupleNMutator = {
            let mutators = mutators.clone();
            Box::new(move |n: usize| ts!(mutators "::tuples::" ident!("Tuple" n "Mutator")))
        };

        let fuzzcheck_traits_Mutator = ts!("fuzzcheck::Mutator");

        let fastrand = ts!("fuzzcheck::fastrand");
        let fastrand_Rng = ts!(fastrand "::Rng");

        Self {
            AlternationMutator: ts!(mutators "::alternation::AlternationMutator"),
            Clone: ts!("::std::clone::Clone"),
            Default: ts!("::std::default::Default"),
            DefaultMutator: ts!(mutators "::DefaultMutator"),
            fastrand_Rng,
            mutators: mutators.clone(),
            // fuzzcheck_mutator_traits_Mutator: ts!(mutators fuzzcheck_traits_Mutator),
            fuzzcheck_traits_Mutator,
            Mi_j: Box::new(|i, j| ident!("M" i "_" j)),
            Mi: Box::new(|i| ident!("M" i)),
            mutator_i: Box::new(|i: usize| ident!("mutator_" i)),
            None: none,
            Option,
            PhantomData: ts!("::std::marker::PhantomData"),
            RefTypes: ts!(mutators "::tuples::RefTypes"),
            Some: some,
            ti,
            ti_value,
            Ti,
            Tuplei,
            TupleMutator,
            TupleMutatorWrapper: ts!(mutators "::tuples::TupleMutatorWrapper"),
            TupleN_ident: ident!("Tuple" n),
            TupleN_path: ts!(mutators "::tuples::" ident!("Tuple" n)),
            TupleNMutator,
            TupleNMutator_ident: ident!("Tuple" n "Mutator"),
            TupleStructure,
            UnitMutator: ts!(mutators "::unit::UnitMutator"),
            Vec: ts!("::std::vec::Vec"),
            VoseAlias: ts!(mutators "::vose_alias::VoseAlias"),
            RecursiveMutator: ts!(mutators "::recursive::RecursiveMutator"),
            Box: ts!("::std::boxed::Box"),
            NeverMutator: ts!("::fuzzcheck::mutators::never::NeverMutator"),
        }
    }
}

fn has_ignore_variant_attribute(attribute: TokenStream) -> bool {
    let mut parser = TokenParser::new(attribute);
    if parser.eat_punct('#').is_none() {
        return false;
    }
    let content = match parser.eat_group(Delimiter::Bracket) {
        Some(proc_macro2::TokenTree::Group(group)) => group,
        None | Some(_) => return false,
    };
    let mut parser = TokenParser::new(content.stream());
    parser.eat_ident("ignore_variant").is_some()
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
