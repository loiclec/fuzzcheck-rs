use decent_synquote_alternative as synquote;
use proc_macro2::{Ident, Span};

use synquote::parser::*;
use synquote::token_builder::*;

use crate::structs_and_enums::CreateWrapperMutatorParams;
use crate::structs_and_enums::{FieldMutator, FieldMutatorKind};
use crate::Common;
use crate::MakeMutatorSettings;

#[allow(non_snake_case)]
pub(crate) fn impl_default_mutator_for_enum(tb: &mut TokenBuilder, enu: &Enum, settings: &MakeMutatorSettings) {
    let cm = Common::new(0);

    let field_mutators = enu
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| match item.get_struct_data() {
            Some((_, fields)) if !fields.is_empty() => fields
                .iter()
                .enumerate()
                .map(|(j, field)| {
                    let mut mutator = None;
                    for attribute in field.attributes.iter() {
                        if let Some((m, init)) = super::read_field_default_mutator_attribute(attribute.clone()) {
                            mutator = Some((m, init));
                        }
                    }
                    if let Some(m) = mutator {
                        FieldMutator {
                            i,
                            j: Some(j),
                            field: field.clone(),
                            kind: FieldMutatorKind::Prescribed(m.0, m.1),
                        }
                    } else {
                        FieldMutator {
                            i,
                            j: Some(j),
                            field: field.clone(),
                            kind: FieldMutatorKind::Generic,
                        }
                    }
                })
                .collect::<Vec<_>>(),
            _ => {
                vec![]
            }
        })
        .collect::<Vec<_>>();

    let TupleNMutator = cm.TupleNMutator.as_ref();
    let EnumSingleVariant = ident!(enu.ident "SingleVariant");

    let InnerMutator = ts!(
        cm.AlternationMutator "<"
        enu.ident enu.generics.removing_bounds_and_eq_type() ","
            EnumSingleVariant "<"
                join_ts!(field_mutators.iter(), item_field_mutators,
                    if item_field_mutators.is_empty() {
                        ts!(cm.UnitMutator "<()>")
                    } else {
                        ts!(
                            TupleNMutator(item_field_mutators.len()) "<"
                                join_ts!(item_field_mutators.iter(), fm,
                                    fm.field.ty ","
                                )
                                join_ts!(item_field_mutators.iter(), fm,
                                    fm.mutator_stream(&cm)
                                , separator: ",")
                            ">"
                        )
                    }
                , separator: ",")
            ">"
        ">"
    );

    let params = CreateWrapperMutatorParams {
        cm: &cm,
        visibility: &enu.visibility,
        type_ident: &enu.ident,
        type_generics: &enu.generics,
        type_where_clause: &enu.where_clause,
        field_mutators: &field_mutators,
        InnerMutator: &InnerMutator,
        new_impl: &ts!("
            pub fn new("
            join_ts!(field_mutators.iter().filter(|fields| !fields.is_empty()).flatten(), field_mutator,
                ident!("mutator_" enu.items[field_mutator.i].ident "_" field_mutator.field.access()) ":" field_mutator.mutator_stream(&cm)
            , separator: ",") ") -> Self {
                Self {
                    mutator: " cm.AlternationMutator "::new(vec!["
                        join_ts!(enu.items.iter().enumerate(), (i, item),
                        EnumSingleVariant "::" item.ident "("
                        match item.get_struct_data() {
                            Some((_, fields)) if !fields.is_empty() =>
                               ts!(
                                    TupleNMutator(fields.len()) "::new("
                                        join_ts!(fields.iter(), field,
                                            ident!("mutator_" enu.items[i].ident "_" field.access())
                                        , separator: ",")
                                    ")"
                               ),
                            _ => ts!(
                                cm.UnitMutator "::default()"
                            )
                        }
                        ")"
                        , separator: ",")
                    "])
                }
            }"
        ),
        default_impl: &ts!("
            fn default() -> Self {
                Self::new("
                join_ts!(&enu.items, item,
                    match item.get_struct_data() {
                        Some((_, fields)) if !fields.is_empty() => join_ts!(fields, _, "<_>::default() ,"),
                        _ => ts!()
                    }
                ) ")
            }
        "),
        settings,
    };

    extend_ts!(tb, crate::structs_and_enums::make_mutator_type_and_impl(params))
}

#[allow(non_snake_case)]
pub(crate) fn impl_basic_enum_structure(tb: &mut TokenBuilder, enu: &Enum, settings: &MakeMutatorSettings) {
    assert!(
        enu.items.len() > 0
            && enu
                .items
                .iter()
                .all(|item| !matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if fields.len() > 0))
    );

    let BasicEnumStructure = ts!(settings.fuzzcheck_mutators_crate "::enums::BasicEnumStructure");

    let items_init = enu
        .items
        .iter()
        .map(|item| match &item.data {
            Some(EnumItemData::Struct(kind, _)) => ts!(kind.open() kind.close()),
            _ => ts!(),
        })
        .collect::<Box<_>>();

    extend_ts!(tb,
        "impl" BasicEnumStructure "for" enu.ident "{
            fn from_item_index(item_index: usize) -> Self {
                match item_index {"
                join_ts!(enu.items.iter().enumerate(), (i, item),
                    i "=>" enu.ident "::" item.ident items_init[i] ","
                )
                "
                    _ => unreachable!()
                }
            }
        
            fn get_item_index(&self) -> usize {
                match self {"
                join_ts!(enu.items.iter().enumerate(), (i, item),
                    enu.ident "::" item.ident items_init[i] "=>" i ","
                )
                "}
            }
        }"
    )
}

#[allow(non_snake_case)]
pub(crate) fn impl_default_mutator_for_basic_enum(tb: &mut TokenBuilder, enu: &Enum, settings: &MakeMutatorSettings) {
    assert!(
        enu.items.len() > 0
            && enu
                .items
                .iter()
                .all(|item| !matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if fields.len() > 0))
    );

    let cm = Common::new(0);

    let BasicEnumMutator = ts!(settings.fuzzcheck_mutators_crate "::enums::BasicEnumMutator");

    extend_ts!(tb,
        "impl" cm.DefaultMutator "for " enu.ident " {
            type Mutator = " BasicEnumMutator ";
        
            fn default_mutator() -> Self::Mutator {
                Self::Mutator::new::<" enu.ident ">()
            }
        }"
    )
}

#[cfg(test)]
mod test {
    use decent_synquote_alternative::{parser::TokenParser, token_builder::TokenBuilder};
    use proc_macro2::TokenStream;

    use super::{impl_basic_enum_structure, impl_default_mutator_for_basic_enum, impl_default_mutator_for_enum};

    #[test]
    fn test_impl_default_mutator_for_enum() {
        let code = "
        pub enum Y {
            Y { y: Option<u8>, z: () },
        }        
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();

        let mut tb = TokenBuilder::new();
        crate::single_variant::make_single_variant_mutator(&mut tb, &enu);
        impl_default_mutator_for_enum(&mut tb, &enu, &<_>::default());
        let generated = tb.end().to_string();

        let expected = "
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_default_mutator_for_basic_enum() {
        let code = "
        enum X {
            A,
            B,
            C,
        }
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();

        let mut tb = TokenBuilder::new();
        impl_default_mutator_for_basic_enum(&mut tb, &enu, &<_>::default());
        let generated = tb.end().to_string();

        let expected = "
        impl fuzzcheck_mutators::DefaultMutator for X {
            type Mutator = fuzzcheck_mutators::enums::BasicEnumMutator;
        
            fn default_mutator() -> Self::Mutator {
                Self::Mutator::default()
            }
        }
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_basic_enum_structure() {
        let code = "
        enum X {
            A,
            B { },
            C ( ),
        }
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();

        let mut tb = TokenBuilder::new();
        impl_basic_enum_structure(&mut tb, &enu, &<_>::default());
        let generated = tb.end().to_string();

        let expected = "
        impl fuzzcheck_mutators::enums::BasicEnumStructure for X {
            fn from_item_index(item_index: usize) -> Self {
                match item_index {
                    0 => X::A,
                    1 => X::B { },
                    2 => X::C ( ),
                    _ => unreachable!()
                }
            }
        
            fn get_item_index(&self) -> usize {
                match self {
                    X::A => 0,
                    X::B { } => 1,
                    X::C ( ) => 2,
                }
            }
        }
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }
}
