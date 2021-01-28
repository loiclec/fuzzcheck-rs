#![allow(non_snake_case)]
use decent_synquote_alternative::{self as synquote, parser::EnumItemData};
use enums::impl_default_mutator_for_basic_enum;
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
            tuples::make_tuple_type_structure(&mut tb, nbr_elements, ts!("crate"));
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
            enums::make_basic_enum_mutator(&mut tb, nbr_elements, fuzzcheck_mutators_crate);
            return tb.end().into();
        }
    }
    panic!()
}

#[proc_macro_derive(TupleStructure)]
pub fn derive_tuple_structure(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(item);
    let mut tb = TokenBuilder::new();
    let mut parser = TokenParser::new(input);

    if let Some(s) = parser.eat_struct() {
        let nbr_fields = s.struct_fields.len();
        if nbr_fields == 1 {
            tuples::impl_wrapped_tuple_1_structure(&mut tb, &s, ts!("fuzzcheck_mutators"));
        } else if nbr_fields > 1 {
            tuples::impl_tuple_structure_trait(&mut tb, &s, ts!("fuzzcheck_mutators"));
        } else {
            extend_ts!(&mut tb,
                "compile_error!(\"The TupleStructure macro only works for structs with one or more fields.\");"
            )
        }
    } else if let Some(_) = parser.eat_enumeration() {
        extend_ts!(&mut tb,
            "compile_error!(\"The TupleStructure macro cannot be used on enums.\");"
        )
    } else {
        extend_ts!(&mut tb,
            "compile_error!(\"The item could not be parsed by the TupleStructure macro. Note: only enums are supported.\");"
        )
    }
    return tb.end().into();
}

#[proc_macro_derive(DefaultMutator)]
pub fn derive_default_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(item);
    let mut tb = TokenBuilder::new();
    let mut parser = TokenParser::new(input);

    let fuzzcheck_mutators_crate = ts!("::fuzzcheck_mutators");

    if let Some(s) = parser.eat_struct() {
        let nbr_fields = s.struct_fields.len();
        if nbr_fields == 0 {
            tuples::impl_default_mutator_for_struct_with_0_field(&mut tb, &s, fuzzcheck_mutators_crate.clone());
        }
        else if nbr_fields == 1 {
            tuples::impl_wrapped_tuple_1_structure(&mut tb, &s, fuzzcheck_mutators_crate.clone());
            tuples::impl_default_mutator_for_struct_with_1_field(&mut tb, &s, fuzzcheck_mutators_crate.clone());
        }
        else {
            tuples::impl_tuple_structure_trait(&mut tb, &s, fuzzcheck_mutators_crate.clone());
            tuples::impl_default_mutator_for_struct_with_more_than_1_field(&mut tb, &s, fuzzcheck_mutators_crate.clone());
        }
    } else if let Some(e) = parser.eat_enumeration() {
        if e.items.len() == 1 && matches!(&e.items[0].data, Some(EnumItemData::Struct(_, fields)) if fields.len() == 1) {
            enums::impl_wrapped_tuple_1_structure(&mut tb, &e, fuzzcheck_mutators_crate.clone());
            enums::impl_default_mutator_for_enum_wrapped_tuple(&mut tb, &e, fuzzcheck_mutators_crate);
        } else if e.items.iter().any(|item| matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if fields.len() > 0)) {
            enums::impl_enum_structure_trait(&mut tb, &e, fuzzcheck_mutators_crate.clone());
            enums::impl_default_mutator_for_enum(&mut tb, &e, fuzzcheck_mutators_crate.clone());
        } else if e.items.len() > 0 { // no associated data anywhere
            enums::impl_basic_enum_structure(&mut tb, &e, fuzzcheck_mutators_crate.clone());
            enums::impl_default_mutator_for_basic_enum(&mut tb, &e, fuzzcheck_mutators_crate);
        } else {
            extend_ts!(&mut tb,
                "compile_error!(\"The DefaultMutator derive proc_macro does not work on empty enums.\");"
            );
        }
    } else {
        extend_ts!(&mut tb,
            "compile_error!(\"The item could not be parsed by the DefaultMutator macro. Note: only enums and structs are supported.\");"
        );
    }
    tb.end().into()
}

#[proc_macro_derive(EnumNPayloadStructure)]
pub fn derive_enum_n_payload_structure(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(item);
    let mut tb = TokenBuilder::new();
    let mut parser = TokenParser::new(input);

    if let Some(e) = parser.eat_enumeration() {
        if e.items.iter().any(|item| matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if fields.len() > 0)) {
            enums::impl_enum_structure_trait(&mut tb, &e, ts!("fuzzcheck_mutators"));
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
    tb.end().into()
}
