use std::collections::HashMap;

use proc_macro2::{Ident, TokenStream};
use syn::{parse2, DataEnum, GenericParam, Generics, TypeParam, Variant, Visibility};

use crate::token_builder::{
    access_field, extend_ts, generics_arg_by_mutating_type_params, ident, join_ts, pattern_match, safe_field_ident, ts,
    TokenBuilder,
};
use crate::{q, Common};

pub fn make_single_variant_mutator(
    tb: &mut TokenBuilder,
    ident: &Ident,
    generics: &Generics,
    vis: &Visibility,
    enu: &DataEnum,
) {
    let cm = Common::new(0);

    let EnumSingleVariant = ident!(ident "SingleVariant");

    // let EnumSingleVariantMutator = ident!(enum_ident "SingleVariantMutator");
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
        for variant in &enu.variants {
            let fields = variant.fields.iter().collect::<Vec<_>>();
            if !fields.is_empty() {
                item_fields.insert(variant.ident.clone(), fields.iter().map(|x| ts!(q!(&x.ty))).collect());
                let field_tys = join_ts!(fields.iter(), field, field.ty, separator: ",");
                map.insert(
                    variant.ident.clone(),
                    ts!(
                        cm.TupleMutator "< (" field_tys ",) ," Tuplei(fields.len()) "<" field_tys "> >"
                    ),
                );
                bindings.insert(
                    variant.ident.clone(),
                    fields
                        .iter()
                        .enumerate()
                        .map(|(idx, field)| safe_field_ident(field, idx))
                        .collect(),
                );
            } else {
                item_fields.insert(variant.ident.clone(), vec![]);
                map.insert(
                    variant.ident.clone(),
                    ts!(
                        cm.TupleMutator "< () ," Tuplei(0) " >"
                    ),
                );
                bindings.insert(variant.ident.clone(), vec![]);
            }
        }
        (item_fields, map, bindings)
    };

    // Generics like:
    // <MSome, MNone>
    let single_variant_generics = {
        let generic_params: Vec<GenericParam> = enu
            .variants
            .iter()
            .map(|variant| {
                let tp: TypeParam = ident!("M" variant.ident).into();
                let gp: GenericParam = tp.into();
                gp
            })
            .collect();
        let mut g = Generics::default();
        g.params.extend(generic_params);
        g
    };
    let mut generics = generics.clone();
    // add more conditions to the enum's generics
    // enum generics with additional condition for each type parameter
    // where T: Clone + 'static
    for tp in generics.type_params_mut() {
        tp.bounds.push(parse2(cm.Clone.clone()).unwrap());
        tp.bounds.push(parse2(ts!("'static")).unwrap());
    }

    // The generics for the impl of the mutator
    // it contains all the generics of the enum (with additional Clone and 'static bounds)
    // as well as its where clause, PLUS the single-variant-generics created earlier PLUS the requirement that each single-variant generics correspond to the correct mutator
    // e.g.
    // <T: Clone + 'static, U: Clone + 'static, MSome: TupleMutator<u8, Tuple1<(u8,)>>, MNone: TupleMutator<(), Tuple0>> ... where T: Default,

    let impl_mutator_generics = {
        let mut g = Generics::default();
        for param in generics.type_params() {
            let tp: TypeParam = param.clone();
            g.params.push(tp.into());
        }
        for variant in enu.variants.iter() {
            // same ident as the single-variant generics
            let mut param: TypeParam = ident!("M" variant.ident).into();
            // with an additional TupleMutator clause
            param
                .bounds
                .push(parse2(item_mutators[&variant.ident].clone()).unwrap());
            g.params.push(param.into());
        }
        g.where_clause = generics.where_clause.clone();
        g
    };

    let pattern_match_binding_append = ident!("__proc_macro__binding__");
    let variant_pattern_match_bindings_to_tuple = |variant_ident| {
        if item_fields[variant_ident].is_empty() {
            ts!("()")
        } else {
            ts!("("
                join_ts!(item_pattern_match_bindings[variant_ident].iter(), binding,
                    ident!(binding pattern_match_binding_append) ","
                )
                ")"
            )
        }
    };
    let variant_pattern_match_bindings_to_enum_variant = |variant: &Variant| {
        ts!(
            ident "::" variant.ident "{"

            join_ts!(variant.fields.iter().enumerate(), (i, field),
                access_field(field, i) ": v." i
            , separator: ",")

            "}"
        )
    };

    let (mutator_gen_impl, _, mutator_gen_where_clause) = impl_mutator_generics.split_for_impl();
    let (_, enum_generics_ty, _) = generics.split_for_impl();

    let selfty = ts!(ident q!(&enum_generics_ty));

    extend_ts!(tb,
    "
    #[derive(" {&cm.Clone} ")]
    #[doc(hidden)]" 
    q!(vis) "enum " EnumSingleVariant q!(&single_variant_generics) "{"
    join_ts!(&enu.variants, item,
        item.ident "(" ident!("M" item.ident) "),"
    )
    "}
    #[allow(non_shorthand_field_patterns)]
    impl " q!(&mutator_gen_impl) cm.fuzzcheck_traits_Mutator "<" selfty "> 
        for " EnumSingleVariant q!(&single_variant_generics) q!(&mutator_gen_where_clause) 
    "{
        #[doc(hidden)]
        type Cache = " EnumSingleVariant
            q!(&generics_arg_by_mutating_type_params(&single_variant_generics, |tp| {
                ts!(tp "::Cache")
            })) ";
        #[doc(hidden)]
        type MutationStep = " EnumSingleVariant
            q!(&generics_arg_by_mutating_type_params(&single_variant_generics, |tp| {
                ts!(tp "::MutationStep")
            })) ";
        #[doc(hidden)]
        type ArbitraryStep = " EnumSingleVariant
            q!(&generics_arg_by_mutating_type_params(&single_variant_generics, |tp| {
                ts!(tp "::ArbitraryStep")
            })) ";
        #[doc(hidden)]
        type UnmutateToken = " EnumSingleVariant
            q!(&generics_arg_by_mutating_type_params(&single_variant_generics, |tp| {
                ts!(tp "::UnmutateToken")
            })) ";

        #[doc(hidden)]
        #[coverage(off)]
        fn initialize(&self) {
            match self {"
                join_ts!(&enu.variants, variant,
                    EnumSingleVariant "::" variant.ident "(m) => { m.initialize() }"
                )
            "}
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
            match self {"
                join_ts!(&enu.variants, variant,
                    EnumSingleVariant "::" variant.ident "(m) =>" EnumSingleVariant "::" variant.ident "(m.default_arbitrary_step()),"
                )
            "}
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn is_valid(&self, value: &" selfty ") -> bool {"
            "match (self, value) {"
            join_ts!(&enu.variants, variant,
                "(" EnumSingleVariant "::" variant.ident "(m)," pattern_match(variant, ident, Some(pattern_match_binding_append.clone())) ") => {
                    m.is_valid(" variant_pattern_match_bindings_to_tuple(&variant.ident) ")
                }"
            )" _ => false,
            }
        }


        #[doc(hidden)]
        #[coverage(off)]
        fn validate_value(&self, value: &" selfty ") -> " cm.Option "<Self::Cache> {
            match (self, value) {"
            join_ts!(&enu.variants, variant,
                "(" EnumSingleVariant "::" variant.ident "(m)," pattern_match(variant, ident, Some(pattern_match_binding_append.clone())) ") => {
                    m.validate_value(" variant_pattern_match_bindings_to_tuple(&variant.ident) ").map(" EnumSingleVariant "::" variant.ident ")
                }"
            )" _ => " cm.None ",
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn default_mutation_step(&self, value: &" selfty ", cache: &Self::Cache) -> Self::MutationStep {
            match (self, value, cache) {"
            join_ts!(&enu.variants, variant,
                "(
                    " EnumSingleVariant ":: " variant.ident " (m) ,
                    " pattern_match(variant, ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant ":: " variant.ident " (c) 
                 ) => {
                     " EnumSingleVariant "::" variant.ident "(m.default_mutation_step(" variant_pattern_match_bindings_to_tuple(&variant.ident) ", c))
                 }"
            )   "_ => unreachable!()
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn global_search_space_complexity(&self) -> f64 {
            match self {"
            join_ts!(&enu.variants, variant,
                EnumSingleVariant "::" variant.ident "(m) => m.global_search_space_complexity() ,"
            )"
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn max_complexity(&self) -> f64 {
            match self {"
            join_ts!(&enu.variants, variant,
                EnumSingleVariant "::" variant.ident "(m) => m.max_complexity() ,"
            )"
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn min_complexity(&self) -> f64 {
            match self {"
            join_ts!(&enu.variants, variant,
                EnumSingleVariant "::" variant.ident "(m) => m.min_complexity() ,"
            )"
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn complexity(&self, value: &" selfty ", cache: &Self::Cache) -> f64 {
            match (self, value, cache) {"
            join_ts!(&enu.variants, variant,
                "(
                    " EnumSingleVariant ":: " variant.ident " (m) ,
                    " pattern_match(variant, ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant ":: " variant.ident " (c) 
                 ) => {
                     m.complexity(" variant_pattern_match_bindings_to_tuple(&variant.ident) ", c) 
                 }"
            )   "_ => unreachable!()
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(" selfty ", f64)> {
            match (self, step) {"
            join_ts!(&enu.variants, variant,
                "(" EnumSingleVariant "::" variant.ident "(m)," EnumSingleVariant "::" variant.ident "(s)) => {"
                    "if let" cm.Some "((v, c)) = m.ordered_arbitrary(s, max_cplx) {
                        " cm.Some "(("
                            variant_pattern_match_bindings_to_enum_variant(variant) ",
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
        #[coverage(off)]
        fn random_arbitrary(&self, max_cplx: f64) -> (" selfty ", f64) {
            match self {"
            join_ts!(&enu.variants, variant,
                EnumSingleVariant "::" variant.ident "(m) => {
                    let (v, c) = m.random_arbitrary(max_cplx);
                    (" 
                        variant_pattern_match_bindings_to_enum_variant(variant) ",
                        c
                    )
                }"
            )"}
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn ordered_mutate(
            &self,
            value: &mut " selfty ",
            cache: &mut Self::Cache,
            step: &mut Self::MutationStep,
            subvalue_provider: &dyn " cm.SubValueProvider ",
            max_cplx: f64,
        ) -> Option<(Self::UnmutateToken, f64)> {
            match (self, value, cache, step) {"
            join_ts!(&enu.variants, variant,
                "(
                    " EnumSingleVariant "::" variant.ident "(m) ,
                    " pattern_match(variant, ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant "::" variant.ident "(c) ,
                    " EnumSingleVariant "::" variant.ident "(s)
                ) => {
                    m.ordered_mutate(" variant_pattern_match_bindings_to_tuple(&variant.ident) ", c, s, subvalue_provider, max_cplx)
                        .map(#[coverage(off)] |(t, c)| (" EnumSingleVariant "::" variant.ident "(t), c))
                }"
            )" _ => unreachable!(),
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn random_mutate(&self, value: &mut " selfty ", cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
            match (self, value, cache) {"
            join_ts!(&enu.variants, variant,
                "(
                    " EnumSingleVariant "::" variant.ident "(m) ,
                    " pattern_match(variant, ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant "::" variant.ident "(c)
                ) => {
                    let (t, c) = m.random_mutate(" 
                        variant_pattern_match_bindings_to_tuple(&variant.ident) ", c, max_cplx"
                    ");
                    (" EnumSingleVariant "::" variant.ident "(t), c)
                }"
            )   "_ => unreachable!()"
            "}
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn unmutate(&self, value: &mut " selfty ", cache: &mut Self::Cache, t: Self::UnmutateToken) {
            match (self, value, cache, t) {"
            join_ts!(&enu.variants, variant,
                "(
                    " EnumSingleVariant "::" variant.ident "(m) ,
                    " pattern_match(variant, ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant "::" variant.ident "(c) ,
                    " EnumSingleVariant "::" variant.ident "(t)
                ) => {"
                    "m.unmutate(" variant_pattern_match_bindings_to_tuple(&variant.ident) ", c, t)"
                "}"
            )" _ => unreachable!()
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn visit_subvalues<'__fuzzcheck_derive_lt>(&self, value: &'__fuzzcheck_derive_lt " selfty ", cache: &'__fuzzcheck_derive_lt Self::Cache, visit: &mut dyn FnMut(&'__fuzzcheck_derive_lt dyn " cm.Any ", f64)) {
            match (self, value, cache) {"
            join_ts!(&enu.variants, variant,
                "(
                    " EnumSingleVariant "::" variant.ident "(m) ,
                    " pattern_match(variant, ident, Some(pattern_match_binding_append.clone())) ",
                    " EnumSingleVariant "::" variant.ident "(cache)
                ) => {
                    m.visit_subvalues(" variant_pattern_match_bindings_to_tuple(&variant.ident) ", cache, visit);
                }"
            )" _ => unreachable!()
            }
        }
    }
    ");
}
