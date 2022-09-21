#![allow(non_snake_case)]
#![allow(clippy::type_complexity)]
#![allow(clippy::large_enum_variant)]
#![feature(stmt_expr_attributes)]
#![feature(let_chains)]

use proc_macro2::{Ident, Literal, TokenStream, TokenTree};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, parse2, parse_macro_input, token, Attribute, DeriveInput, Error, LitBool, Token};
use token_builder::{extend_ts, ident, ts, TokenBuilder};

mod enums;
mod single_variant;
mod structs_and_enums;

mod token_builder;

mod tuples;

macro_rules! q {
    ($part:expr) => {
        $crate::token_builder::Quoted(&$part)
    };
}
pub(crate) use q;

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
    let literal = parse_macro_input!(item as Literal);
    if let Ok(nbr_elements) = literal.to_string().parse::<usize>() {
        let mut tb = TokenBuilder::default();
        tuples::make_basic_tuple_mutator(&mut tb, nbr_elements);
        tb.finish().into()
    } else {
        ts!(
            "compile_error!("
                q!("make_basic_tuple_mutator expects a small positive integer as argument")
            "]);"
        )
        .into()
    }
}

#[doc(hidden)]
#[proc_macro_derive(TupleStructure)]
pub fn derive_tuple_structure(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as DeriveInput);
    if let syn::Data::Struct(s) = item.data {
        let mut tb = TokenBuilder::default();
        let nbr_fields = s.fields.len();
        if nbr_fields > 0 {
            tuples::impl_tuple_structure_trait(&mut tb, &item.ident, &item.generics, &s);
            return tb.finish().into();
        }
    }
    ts!(
        "compile_error!("
            q!("The TupleStructure macro only works for structs with one or more fields.")
        "]);"
    )
    .into()
}
#[proc_macro]
pub fn make_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let settings = parse_macro_input!(item as MakeMutatorSettings);
    // let item = proc_macro2::TokenStream::from(item);
    derive_default_mutator_(settings).into()
}

#[proc_macro_derive(DefaultMutator, attributes(field_mutator, ignore_variant))]
pub fn derive_default_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let settings = MakeMutatorSettings {
        name: None,
        recursive: false,
        default: true,
        ty: parse_macro_input!(item as DeriveInput),
    };
    derive_default_mutator_(settings).into()
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

fn derive_default_mutator_(settings: MakeMutatorSettings) -> proc_macro2::TokenStream {
    let mut tb = TokenBuilder::default();
    let item = settings.ty.clone();
    match item.data {
        syn::Data::Struct(s) => {
            let nbr_fields = s.fields.len();
            if nbr_fields == 0 {
                tuples::impl_default_mutator_for_struct_with_0_field(&mut tb, &item.ident, &s);
            } else {
                tuples::impl_tuple_structure_trait(&mut tb, &item.ident, &item.generics, &s);
                tuples::impl_default_mutator_for_struct(&mut tb, &item.ident, &item.generics, &item.vis, &s, &settings);
            }
        }
        syn::Data::Enum(e) => {
            if e.variants.iter().any(|variant| match &variant.fields {
                syn::Fields::Named(fs) => !fs.named.is_empty(),
                syn::Fields::Unnamed(fs) => !fs.unnamed.is_empty(),
                syn::Fields::Unit => false,
            }) {
                single_variant::make_single_variant_mutator(&mut tb, &item.ident, &item.generics, &item.vis, &e);
                enums::impl_default_mutator_for_enum(&mut tb, &item.ident, &item.generics, &item.vis, &e, &settings);
            } else if !e.variants.is_empty() {
                // no associated data anywhere
                enums::impl_basic_enum_structure(&mut tb, &item.ident, &e);
                enums::impl_default_mutator_for_basic_enum(&mut tb, &item.ident, &e);
            } else {
                extend_ts!(
                    &mut tb,
                    "compile_error!(" q!("The DefaultMutator derive proc_macro does not work on empty enums.") ");"
                );
            }
        }
        syn::Data::Union(_) => {
            extend_ts!(
                &mut tb,
                "compile_error!(" q!("Unions are not supported by fuzzcheck’s procedural macros.") ");"
            );
        }
    }
    tb.finish()
}

fn make_single_variant_mutator_(item: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    match parse2::<DeriveInput>(item) {
        Ok(e) => match e.data {
            syn::Data::Enum(enum_data) => {
                let mut tb = TokenBuilder::default();
                single_variant::make_single_variant_mutator(&mut tb, &e.ident, &e.generics, &e.vis, &enum_data);
                tb.finish()
            }
            _ => {
                ts!(
                    "compile_error!(" q!("make_single_variant_mutator only works for enums") ");"
                )
            }
        },
        Err(e) => e.to_compile_error(),
    }
}

/* common */

struct MakeMutatorSettings {
    name: Option<proc_macro2::Ident>,
    recursive: bool,
    default: bool,
    ty: DeriveInput,
}

impl Parse for MakeMutatorSettings {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut recursive = None;
        let mut default = None;

        while !input.is_empty() {
            let ident = input.call(Ident::parse_any)?;
            match ident.to_string().as_str() {
                "name" => {
                    let _ = input.parse::<Token![:]>()?;
                    name = Some(input.parse::<Ident>()?);
                }
                "recursive" => {
                    let _ = input.parse::<Token![:]>()?;
                    let value = input.parse::<LitBool>()?;
                    recursive = Some(value.value);
                }
                "default" => {
                    let _ = input.parse::<Token![:]>()?;
                    let value = input.parse::<LitBool>()?;
                    default = Some(value.value);
                }
                "type" => {
                    let _ = input.parse::<Token![:]>()?;
                    let ty = input.parse::<DeriveInput>()?;

                    return Ok(MakeMutatorSettings {
                        name,
                        recursive: recursive.unwrap_or(false),
                        default: default.unwrap_or(true),
                        ty,
                    });
                }
                x => {
                    return Err(Error::new(
                        ident.span(),
                        &format!("{x} is not a valid setting of make_muattor"),
                    ));
                }
            }
            let _ = input.parse::<Token![,]>()?;
        }
        Err(Error::new(input.span(), "make_mutator requires a `type` argument"))
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
    Any: TokenStream,
    Clone: TokenStream,
    Default: TokenStream,
    DefaultMutator: TokenStream,
    CrossoverStep: TokenStream,
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
    SubValueProvider: TokenStream,
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
            Box::new(move |i: usize| ts!(&mutators "::tuples::" ident!("Tuple" i)))
        };
        let TupleNMutator = {
            let mutators = mutators.clone();
            Box::new(move |n: usize| ts!(&mutators "::tuples::" ident!("Tuple" n "Mutator")))
        };

        let fuzzcheck_traits_Mutator = ts!("fuzzcheck::Mutator");

        let fastrand = ts!("fuzzcheck::fastrand");
        let fastrand_Rng = ts!(fastrand "::Rng");

        Self {
            AlternationMutator: ts!(mutators "::alternation::AlternationMutator"),
            Any: ts!("::std::any::Any"),
            Clone: ts!("::std::clone::Clone"),
            CrossoverStep: ts!("fuzzcheck::mutators::CrossoverStep"),
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
            SubValueProvider: ts!("fuzzcheck::SubValueProvider"),
        }
    }
}

fn has_ignore_variant_attribute(attribute: &Attribute) -> bool {
    if let Some(ident) = attribute.path.get_ident() && ident == "ignore_variant" {
        true
    } else {
        false
    }
}

struct FieldMutatorAttribute {
    ty: syn::Type,
    equal: Option<TokenStream>,
}
impl syn::parse::Parse for FieldMutatorAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let _ = parenthesized!(content in input);
        let input = content;

        let ty = input.parse::<syn::Type>()?;
        if input.is_empty() {
            return Ok(Self { ty, equal: None });
        }
        if !input.peek(Token![=]) {
            return Err(syn::Error::new(
                input.span(),
                "Expected '=' (or nothing) after the type of field_mutator",
            ));
        }
        let _ = input.parse::<TokenTree>().unwrap();
        if !input.peek(token::Brace) {
            return Err(syn::Error::new(
                input.span(),
                "Expected a block delimited by braces containing the expression that initialises the field mutator",
            ));
        }
        let x = input.parse::<TokenTree>().unwrap();
        if let TokenTree::Group(g) = x {
            Ok(Self {
                ty,
                equal: Some(g.stream()),
            })
        } else {
            unreachable!()
        }
    }
}

fn read_field_default_mutator_attribute(attribute: &Attribute) -> Result<Option<FieldMutatorAttribute>, syn::Error> {
    if let Some(ident) = attribute.path.get_ident() {
        if ident != "field_mutator" {
            return Ok(None);
        }
        parse2::<FieldMutatorAttribute>(attribute.tokens.clone()).map(Some)
    } else {
        Ok(None)
    }
}

// #[cfg(test)]
// mod tests {
//     use syn::{parse2, DeriveInput};

//     use crate::read_field_default_mutator_attribute;
//     use crate::token_builder::ts;

//     #[test]
//     fn test_att() {
//         let tokens = quote::quote! {
//             #[field_mutator(u8)]
//             struct S;
//         };
//         let s = parse2::<DeriveInput>(tokens).unwrap();
//         let attr = &s.attrs[0];
//         let res = read_field_default_mutator_attribute(attr);
//         match res {
//             Ok(Some(x)) => println!(""),
//             Ok(None) => println!("none"),
//             Err(e) => println!("{}", ts!(e.to_compile_error())),
//         }
//     }
// }
