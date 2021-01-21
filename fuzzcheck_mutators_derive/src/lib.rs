use decent_synquote_alternative as synquote;
use synquote::token_builder::*;
use synquote::{parser::TokenParser, token_builder::TokenBuilder};

mod tuples;

#[macro_use]
extern crate decent_synquote_alternative;

// make_basic_tuple_mutator!(2) {
//     (A, B, C, D, E, F, G, H, I, J)
// }

#[proc_macro]
pub fn make_basic_tuple_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut tb = TokenBuilder::new();

    let mut parser = TokenParser::new(item.into());
    if let Some(l) = parser.eat_literal() {
        if let Ok(nbr_elements) = l.to_string().parse::<usize>() {
            let fuzzcheck_mutators_crate = parser.eat_type_path().unwrap_or(ts!("fuzzcheck_mutators"));
            tuples::make_basic_tuple_mutator(&mut tb, nbr_elements, fuzzcheck_mutators_crate);
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

#[proc_macro_derive(WrappedStructure)]
pub fn derive_wrapped_structure(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(item);
    let mut tb = TokenBuilder::new();
    let mut parser = TokenParser::new(input);

    if let Some(s) = parser.eat_struct() {
        tuples::impl_wrapped_structure_trait(&mut tb, s, ts!("fuzzcheck_mutators"));
        return tb.end().into();
    }
    panic!()
}

#[proc_macro_derive(TupleNStructure)]
pub fn derive_tuple_n_structure(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(item);
    let mut tb = TokenBuilder::new();
    let mut parser = TokenParser::new(input);

    if let Some(s) = parser.eat_struct() {
        tuples::impl_tuple_structure_trait(&mut tb, s, ts!("fuzzcheck_mutators"));
        return tb.end().into();
    }
    panic!()
}

// #[proc_macro_derive(DefaultMutator)]
// pub fn derive_default_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     let input = proc_macro2::TokenStream::from(item);
//     let mut tb = TokenBuilder::new();
//     let mut parser = TokenParser::new(input);

//     if let Some(s) = parser.eat_struct() {
//         tuples::impl_tuple_structure_trait(&mut tb, s, ts!("fuzzcheck_mutators"));

//         return tb.end().into();
//     }
//     panic!()
// }
