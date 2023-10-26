use proc_macro2::Ident;
use syn::{DataEnum, Generics, Visibility};

use crate::structs_and_enums::{CreateWrapperMutatorParams, FieldMutator, FieldMutatorKind};
use crate::token_builder::{access_field, extend_ts, ident, join_ts, ts, TokenBuilder};
use crate::{q, Common, MakeMutatorSettings};

fn size_to_cplxity(size: usize) -> f64 {
    (usize::BITS - (size.saturating_sub(1)).leading_zeros()) as f64
}
#[allow(non_snake_case)]
pub(crate) fn impl_default_mutator_for_enum(
    tb: &mut TokenBuilder,
    enum_ident: &Ident,
    generics: &Generics,
    vis: &Visibility,
    enu: &DataEnum,
    settings: &MakeMutatorSettings,
) {
    let cm = Common::new(0);

    let field_mutators = enu
        .variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            (
                index,
                variant,
                variant.attrs.iter().any(super::has_ignore_variant_attribute),
            )
        })
        .map(|(i, variant, should_ignore)| {
            if !variant.fields.is_empty() {
                variant
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(j, field)| {
                        if should_ignore {
                            return FieldMutator {
                                i,
                                j: Some(j),
                                field: field.clone(),
                                kind: FieldMutatorKind::Ignore,
                            };
                        }
                        let mut mutator = None;
                        for attribute in field.attrs.iter() {
                            match super::read_field_default_mutator_attribute(attribute) {
                                Ok(Some(field_mutator_attribute)) => {
                                    mutator = Some((field_mutator_attribute.ty, field_mutator_attribute.equal));
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    tb.stream(e.to_compile_error());
                                }
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
                    .collect::<Vec<_>>()
            } else {
                vec![]
            }
        })
        .collect::<Vec<_>>();

    let TupleNMutator = cm.TupleNMutator.as_ref();
    let EnumSingleVariant = ident!(&enum_ident "SingleVariant");

    let (_, generic_args, _) = generics.split_for_impl();
    let selfty = ts!(enum_ident q!(generic_args));

    let InnerMutator = ts!(
        cm.AlternationMutator "<"
            selfty ","
            EnumSingleVariant "<"
                join_ts!(field_mutators.iter(), variant_field_mutators,
                    if variant_field_mutators.is_empty() {
                        ts!(TupleNMutator(0))
                    } else if variant_field_mutators.iter().any(|mutator| {
                        mutator.kind.is_ignore()
                    }) {
                        cm.NeverMutator.clone()
                    } else {
                        ts!(
                            TupleNMutator(variant_field_mutators.len()) "<"
                                join_ts!(variant_field_mutators.iter().filter(|mutator| {
                                    !mutator.kind.is_ignore()
                                }), fm,
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
        visibility: vis,
        type_ident: enum_ident,
        type_generics: generics,
        field_mutators: &field_mutators,
        InnerMutator: &InnerMutator,
        new_impl: &ts!("
            #[coverage(off)]
            pub fn new("
            join_ts!(field_mutators.iter().filter(|fields|
                !fields.is_empty() && fields.iter().all(|field| !field.kind.is_ignore())
            ).flatten(), field_mutator,
                ident!("mutator_" enu.variants[field_mutator.i].ident "_" access_field(&field_mutator.field, field_mutator.j.unwrap())) ":" field_mutator.mutator_stream(&cm)
            , separator: ",") ") -> Self {
                Self {
                    mutator: " cm.AlternationMutator "::new(vec!["
                        join_ts!(enu.variants.iter().enumerate().filter(|(_, variant)| {
                                    variant.attrs.iter().all(|attr| {
                                        !super::has_ignore_variant_attribute(attr)
                                    })
                                }), (i, variant),
                        EnumSingleVariant "::" variant.ident "("
                        if variant.fields.is_empty() {
                            TupleNMutator(0)
                        } else {
                                ts!(
                                    TupleNMutator(variant.fields.len()) "::new("
                                        join_ts!(variant.fields.iter().enumerate(), (idx, field),
                                            ident!("mutator_" enu.variants[i].ident "_" access_field(field, idx))
                                        , separator: ",")
                                    ")"
                               )
                        }
                        ")"
                        , separator: ",")
                    "], " format!("{:.2}", size_to_cplxity(enu.variants.len())) ")
                }
            }"
        ),
        settings,
    };

    extend_ts!(tb, crate::structs_and_enums::make_mutator_type_and_impl(params))
}

#[allow(non_snake_case)]
pub(crate) fn impl_basic_enum_structure(tb: &mut TokenBuilder, enum_ident: &Ident, enu: &DataEnum) {
    assert!(!enu.variants.is_empty() && enu.variants.iter().all(|variant| variant.fields.is_empty()));

    let cm = Common::new(0);

    let BasicEnumStructure = ts!(cm.mutators "::enums::BasicEnumStructure");

    let variants_init = enu
        .variants
        .iter()
        .map(|variant| match variant.fields {
            syn::Fields::Named(_) => ts!("{ }"),
            syn::Fields::Unnamed(_) => ts!("( )"),
            syn::Fields::Unit => ts!(),
        })
        .collect::<Box<_>>();

    let (ignored, not_ignored): (Vec<_>, Vec<_>) = enu
        .variants
        .iter()
        .partition(|variant| variant.attrs.iter().any(super::has_ignore_variant_attribute));

    extend_ts!(tb,
        "impl" BasicEnumStructure "for" enum_ident "{
            #[coverage(off)]
            fn from_variant_index(variant_index: usize) -> Self {
                match variant_index {"
                join_ts!(not_ignored.iter().enumerate(), (i, variant),
                    i "=>" enum_ident "::" variant.ident variants_init[i] ","
                )
                join_ts!(ignored.iter().enumerate(), (i, variant),
                    (i + not_ignored.len()) "=>" enum_ident "::" variant.ident variants_init[i] ","
                )
                "
                    _ => unreachable!()
                }
            }
            #[coverage(off)]
            fn get_variant_index(&self) -> usize {
                match self {"
                join_ts!(not_ignored.iter().enumerate(), (i, variant),
                    enum_ident "::" variant.ident variants_init[i] "=>" i ","
                )
                join_ts!(ignored.iter().enumerate(), (i, variant),
                    enum_ident "::" variant.ident variants_init[i + not_ignored.len()] "=>" i + not_ignored.len() ","
                )
                "}
            }
        }"
    );
}

#[allow(non_snake_case)]
pub(crate) fn impl_default_mutator_for_basic_enum(tb: &mut TokenBuilder, enum_ident: &Ident, enu: &DataEnum) {
    assert!(!enu.variants.is_empty() && enu.variants.iter().all(|variant| variant.fields.is_empty()));

    let cm = Common::new(0);

    let BasicEnumMutator = ts!(cm.mutators "::enums::BasicEnumMutator");

    let count_non_ignored = enu
        .variants
        .iter()
        .filter(|variant| {
            variant
                .attrs
                .iter()
                .all(|attr| !super::has_ignore_variant_attribute(attr))
        })
        .count();

    extend_ts!(tb,
        "impl" cm.DefaultMutator "for " enum_ident " {
            type Mutator = " BasicEnumMutator ";
            #[coverage(off)]
            fn default_mutator() -> Self::Mutator {
                Self::Mutator::new::<" enum_ident ">(" q!(count_non_ignored) ")
            }
        }"
    )
}
