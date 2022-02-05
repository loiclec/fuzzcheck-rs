use std::collections::HashMap;

use decent_synquote_alternative as synquote;
use proc_macro2::{Ident, Span, TokenStream};

use synquote::parser::*;
use synquote::token_builder::*;

use crate::Common;

pub fn make_single_variant_mutator(tb: &mut TokenBuilder, enu: &Enum) {
    let cm = Common::new(0);

    let EnumSingleVariant = ident!(enu.ident "SingleVariant");

    // let EnumSingleVariantMutator = ident!(enu.ident "SingleVariantMutator");
    let Tuplei = cm.Tuplei.as_ref();

    // item_fields: vector holding the item field types
    // item_mutators: the token stream of the tuple mutator for the item fields
    // item_pattern_match_bindings: the bindings made when pattern matching the item
    let (item_fields, item_mutators, item_pattern_match_bindings): (
        HashMap<Ident, Vec<TokenStream>>,
        HashMap<Ident, TokenStream>,
        HashMap<Ident, Vec<Ident>>,
    ) = {
        let mut item_fields = HashMap::new();
        let mut map = HashMap::new();
        let mut bindings = HashMap::new();
        for item in &enu.items {
            match item.get_struct_data() {
                Some((_, fields)) if !fields.is_empty() => {
                    item_fields.insert(item.ident.clone(), fields.iter().map(|x| ts!(x.ty)).collect());
                    let field_tys = join_ts!(fields.iter(), field, field.ty, separator: ",");
                    map.insert(
                        item.ident.clone(),
                        ts!(
                            cm.TupleMutator "< (" field_tys ",) ," Tuplei(fields.len()) "<" field_tys "> >"
                        ),
                    );
                    bindings.insert(
                        item.ident.clone(),
                        fields.iter().map(|field| field.safe_ident()).collect(),
                    );
                }
                _ => {
                    item_fields.insert(item.ident.clone(), vec![]);
                    map.insert(
                        item.ident.clone(),
                        ts!(
                            cm.TupleMutator "< () ," Tuplei(0) " >"
                        ),
                    );
                    bindings.insert(item.ident.clone(), vec![]);
                }
            }
        }
        (item_fields, map, bindings)
    };

    let single_variant_generics_for_prefix = |prefix: &Ident| Generics {
        lifetime_params: vec![],
        type_params: enu
            .items
            .iter()
            .map(|item| TypeParam {
                type_ident: ts!(ident!(prefix item.ident)),
                ..<_>::default()
            })
            .collect(),
    };
    let single_variant_generics = single_variant_generics_for_prefix(&ident!("M"));
    let enum_generics_no_bounds = enu.generics.removing_bounds_and_eq_type();

    let mut enum_where_clause_plus_cond = enu.where_clause.clone().unwrap_or_default();
    enum_where_clause_plus_cond.add_clause_items(join_ts!(&enu.generics.type_params, tp,
        tp.type_ident ":" cm.Clone "+ 'static ,"
    ));
    let impl_mutator_generics = {
        let mut impl_mutator_generics = enu.generics.clone();
        for lp in &single_variant_generics.lifetime_params {
            impl_mutator_generics.lifetime_params.push(lp.clone());
        }
        for tp in &single_variant_generics.type_params {
            impl_mutator_generics.type_params.push(tp.clone());
        }
        impl_mutator_generics
    };
    let mut impl_mutator_where_clause = enum_where_clause_plus_cond.clone();
    impl_mutator_where_clause.add_clause_items(join_ts!(&enu.items, item,
        ident!("M" item.ident) ":" item_mutators[&item.ident] ","
    ));

    let pattern_match_binding_append = ident!("__proc_macro__binding__");
    let item_pattern_match_bindings_to_tuple = |item_ident, _mutable| {
        if item_fields[item_ident].is_empty() {
            ts!("()")
        } else {
            ts!("("
                join_ts!(item_pattern_match_bindings[item_ident].iter(), binding,
                    ident!(binding pattern_match_binding_append) ","
                )
                ")"
            )
        }
    };
    let item_pattern_match_bindings_to_enum_item = |item: &EnumItem| {
        let fields = item.get_struct_data().map(|x| x.1).unwrap_or_default();
        ts!(
            enu.ident "::" item.ident "{"

            join_ts!(fields.iter().enumerate(), (i, field),
                field.access() ": v." i
            , separator: ",")

            "}"
        )
    };

    extend_ts!(tb,
    "
    #[derive(" cm.Clone ")]
    #[doc(hidden)]
    pub enum " EnumSingleVariant single_variant_generics.removing_eq_type() "{"
    join_ts!(&enu.items, item,
        item.ident "(" ident!("M" item.ident) "),"
    )
    "}
    #[allow(non_shorthand_field_patterns)]
    impl " impl_mutator_generics.removing_eq_type() cm.fuzzcheck_traits_Mutator "<" enu.ident enum_generics_no_bounds "> 
        for " EnumSingleVariant single_variant_generics.removing_bounds_and_eq_type() impl_mutator_where_clause 
    "{
        #[doc(hidden)]
        type Cache = " EnumSingleVariant
            single_variant_generics.mutating_type_params(|tp| {
                tp.type_ident = ts!(tp.type_ident "::Cache")
            }) ";
        #[doc(hidden)]
        type MutationStep = " EnumSingleVariant
            single_variant_generics.mutating_type_params(|tp| {
                tp.type_ident = ts!(tp.type_ident "::MutationStep")
            }) ";
        #[doc(hidden)]
        type ArbitraryStep = " EnumSingleVariant
            single_variant_generics.mutating_type_params(|tp| {
                tp.type_ident = ts!(tp.type_ident "::ArbitraryStep")
            }) ";
        #[doc(hidden)]
        type UnmutateToken = " EnumSingleVariant
            single_variant_generics.mutating_type_params(|tp| {
                tp.type_ident = ts!(tp.type_ident "::UnmutateToken")
            }) ";
        #[doc(hidden)]
        type LensPath = " EnumSingleVariant
            single_variant_generics.mutating_type_params(|tp| {
                tp.type_ident = ts!(tp.type_ident "::LensPath")
            }) ";

        #[doc(hidden)]
        #[no_coverage]
        fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
            match self {"
                join_ts!(&enu.items, item,
                    EnumSingleVariant "::" item.ident "(m) =>" EnumSingleVariant "::" item.ident "(m.default_arbitrary_step()),"
                )
            "}
        }

        #[doc(hidden)]
        #[no_coverage]
        fn validate_value(&self, value: &" enu.ident enum_generics_no_bounds ") -> " cm.Option "<Self::Cache> {
            match (self, value) {"
            join_ts!(&enu.items, item,
                "(" EnumSingleVariant "::" item.ident "(m)," item.pattern_match(&enu.ident, Some(pattern_match_binding_append.clone())) ") => {
                    m.validate_value(" item_pattern_match_bindings_to_tuple(&item.ident, false) ").map(" EnumSingleVariant "::" item.ident ")
                }"
            )" _ => " cm.None ",
            }
        }

        #[doc(hidden)]
        #[no_coverage]
        fn default_mutation_step(&self, value: &" enu.ident enum_generics_no_bounds ", cache: &Self::Cache) -> Self::MutationStep {
            match (self, value, cache) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant ":: " item.ident " (m) ,
                    " item.pattern_match(&enu.ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant ":: " item.ident " (c) 
                 ) => {
                     " EnumSingleVariant "::" item.ident "(m.default_mutation_step(" item_pattern_match_bindings_to_tuple(&item.ident, false) ", c))
                 }"
            )   "_ => unreachable!()
            }
        }

        #[doc(hidden)]
        #[no_coverage]
        fn global_search_space_complexity(&self) -> f64 {
            match self {"
            join_ts!(&enu.items, item,
                EnumSingleVariant "::" item.ident "(m) => m.global_search_space_complexity() ,"
            )"
            }
        }

        #[doc(hidden)]
        #[no_coverage]
        fn max_complexity(&self) -> f64 {
            match self {"
            join_ts!(&enu.items, item,
                EnumSingleVariant "::" item.ident "(m) => m.max_complexity() ,"
            )"
            }
        }

        #[doc(hidden)]
        #[no_coverage]
        fn min_complexity(&self) -> f64 {
            match self {"
            join_ts!(&enu.items, item,
                EnumSingleVariant "::" item.ident "(m) => m.min_complexity() ,"
            )"
            }
        }

        #[doc(hidden)]
        #[no_coverage]
        fn complexity(&self, value: &" enu.ident enum_generics_no_bounds ", cache: &Self::Cache) -> f64 {
            match (self, value, cache) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant ":: " item.ident " (m) ,
                    " item.pattern_match(&enu.ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant ":: " item.ident " (c) 
                 ) => {
                     m.complexity(" item_pattern_match_bindings_to_tuple(&item.ident, false) ", c) 
                 }"
            )   "_ => unreachable!()
            }
        }

        #[doc(hidden)]
        #[no_coverage]
        fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(" enu.ident enum_generics_no_bounds ", f64)> {
            match (self, step) {"
            join_ts!(&enu.items, item,
                "(" EnumSingleVariant "::" item.ident "(m)," EnumSingleVariant "::" item.ident "(s)) => {"
                    "if let" cm.Some "((v, c)) = m.ordered_arbitrary(s, max_cplx) {
                        " cm.Some "(("
                            item_pattern_match_bindings_to_enum_item(item) ",
                            c
                        ))
                    } else {
                        None
                    }
                }"
            ) "_ => unreachable!()
            }
        }

        #[doc(hidden)]
        #[no_coverage]
        fn random_arbitrary(&self, max_cplx: f64) -> (" enu.ident enum_generics_no_bounds ", f64) {
            match self {"
            join_ts!(&enu.items, item,
                EnumSingleVariant "::" item.ident "(m) => {
                    let (v, c) = m.random_arbitrary(max_cplx);
                    (" 
                        item_pattern_match_bindings_to_enum_item(item) ",
                        c
                    )
                }"
            )"}
        }
        
        #[doc(hidden)]
        #[no_coverage]
        fn ordered_mutate(
            &self,
            value: &mut " enu.ident enum_generics_no_bounds ",
            cache: &mut Self::Cache,
            step: &mut Self::MutationStep,
            max_cplx: f64,
        ) -> Option<(Self::UnmutateToken, f64)> {
            match (self, value, cache, step) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant "::" item.ident "(m) ,
                    " item.pattern_match(&enu.ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant "::" item.ident "(c) ,
                    " EnumSingleVariant "::" item.ident "(s)
                ) => {
                    m.ordered_mutate(" item_pattern_match_bindings_to_tuple(&item.ident, true) ", c, s, max_cplx)
                        .map(#[no_coverage] |(t, c)| (" EnumSingleVariant "::" item.ident "(t), c))
                }"
            )" _ => unreachable!(),
            }
        }

        #[doc(hidden)]
        #[no_coverage]
        fn random_mutate(&self, value: &mut " enu.ident enum_generics_no_bounds ", cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
            match (self, value, cache) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant "::" item.ident "(m) ,
                    " item.pattern_match(&enu.ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant "::" item.ident "(c)
                ) => {
                    let (t, c) = m.random_mutate(" 
                        item_pattern_match_bindings_to_tuple(&item.ident, true) ", c, max_cplx"
                    ");
                    (" EnumSingleVariant "::" item.ident "(t), c)
                }"
            )   "_ => unreachable!()"
            "}
        }

        #[doc(hidden)]
        #[no_coverage]
        fn unmutate(&self, value: &mut " enu.ident enum_generics_no_bounds ", cache: &mut Self::Cache, t: Self::UnmutateToken) {
            match (self, value, cache, t) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant "::" item.ident "(m) ,
                    " item.pattern_match(&enu.ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant "::" item.ident "(c) ,
                    " EnumSingleVariant "::" item.ident "(t)
                ) => {"
                    "m.unmutate(" item_pattern_match_bindings_to_tuple(&item.ident, true) ", c, t)"
                "}"
            )" _ => unreachable!()
            }
        }
        #[doc(hidden)]
        #[no_coverage]
        fn lens<'a>(&self, value: &'a " enu.ident enum_generics_no_bounds ", cache: &'a Self::Cache, path: &Self::LensPath) -> &'a dyn" cm.Any " 
        {
            match (self, value, cache, path) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant "::" item.ident "(m) ,
                    " item.pattern_match(&enu.ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant "::" item.ident "(cache) ,
                    " EnumSingleVariant "::" item.ident "(path) ,
                ) => {"
                    "
                    m.lens(" item_pattern_match_bindings_to_tuple(&item.ident, true) ", cache, path)
                    "
                "}"
            )" _ => unreachable!()
            }
        }
        #[doc(hidden)]
        #[no_coverage]
        fn all_paths(&self, value: &" enu.ident enum_generics_no_bounds ", cache: &Self::Cache, register_path: &mut dyn FnMut(" cm.TypeId ", Self::LensPath)) {
            match (self, value, cache) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant "::" item.ident "(m) ,
                    " item.pattern_match(&enu.ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant "::" item.ident "(cache)
                ) => {
                    m.all_paths(" item_pattern_match_bindings_to_tuple(&item.ident, true) ", cache, #[no_coverage] &mut |typeid, subpath| {
                        register_path(typeid, " EnumSingleVariant "::" item.ident "(subpath));
                    });
                }"
            )" _ => unreachable!()
            }
        }
        #[doc(hidden)]
        #[no_coverage]
        fn crossover_mutate<'a>(
            &self,
            value: &mut " enu.ident enum_generics_no_bounds ", 
            cache: &mut Self::Cache,
            subvalue_provider: &dyn " cm.SubValueProvider ",
            max_cplx: f64,
        ) -> (Self::UnmutateToken, f64) {
            match (self, value, cache) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant "::" item.ident "(m) ,
                    " item.pattern_match(&enu.ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant "::" item.ident "(c)
                ) => {
                    let (unmutate, cplx) = m.crossover_mutate(" 
                        item_pattern_match_bindings_to_tuple(&item.ident, true) ", c, subvalue_provider, max_cplx"
                    ");
                    (" EnumSingleVariant "::" item.ident "(unmutate), cplx)
                }"
            )   "_ => unreachable!()"
            "}
        }
    }
    ");
}
