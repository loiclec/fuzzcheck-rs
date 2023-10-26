use proc_macro2::Ident;
use syn::{parse2, DataStruct, Generics, Visibility, WhereClause};

use crate::structs_and_enums::{FieldMutator, FieldMutatorKind};
use crate::token_builder::*;
use crate::{q, Common, MakeMutatorSettings};

pub fn make_basic_tuple_mutator(tb: &mut TokenBuilder, nbr_elements: usize) {
    make_tuple_type_structure(tb, nbr_elements);

    declare_tuple_mutator(tb, nbr_elements);
    declare_tuple_mutator_helper_types(tb, nbr_elements);
    impl_mutator_trait(tb, nbr_elements);

    impl_default_mutator_for_tuple(tb, nbr_elements);
}

#[allow(non_snake_case)]
pub fn make_tuple_type_structure(tb: &mut TokenBuilder, nbr_elements: usize) {
    let cm = Common::new(nbr_elements);
    let Ti = cm.Ti.as_ref();

    // T0, T1, ...
    let type_params = join_ts!(0..nbr_elements, i, Ti(i), separator: ",");
    let type_params_static_bound = join_ts!(0..nbr_elements, i, Ti(i) ": 'static", separator: ",");
    let tuple_owned = ts!("(" type_params ",)");
    let tuple_ref = ts!("(" join_ts!(0..nbr_elements, i, "&'__fuzzcheck_derive_lt" Ti(i) ",") ")");
    let tuple_mut = ts!("(" join_ts!(0..nbr_elements, i, "&'__fuzzcheck_derive_lt mut" Ti(i) ",") ")");

    let PhantomData = ts!(cm.PhantomData "<(" type_params ",)>");

    extend_ts!(tb,
        "#[doc = "
            q!(format!("A marker type implementing [`RefTypes`](crate::mutators::tuples::RefTypes) indicating that a type has the [structure](crate::mutators::tuples::TupleStructure) of a {nbr_elements}-tuple."))
        "]"
        "pub struct" cm.TupleN_ident "<" type_params_static_bound "> {
            _phantom: " PhantomData ",
        }
        impl<" type_params_static_bound "> " cm.RefTypes " for " cm.TupleN_ident "<" type_params "> {
            type Owned = " tuple_owned ";
            type Ref<'__fuzzcheck_derive_lt> = " tuple_ref ";
            type Mut<'__fuzzcheck_derive_lt> = " tuple_mut ";
            #[coverage(off)]
            fn get_ref_from_mut<'__fuzzcheck_derive_lt>(v: &'__fuzzcheck_derive_lt Self::Mut<'__fuzzcheck_derive_lt>) -> Self::Ref<'__fuzzcheck_derive_lt> {
                (" join_ts!(0..nbr_elements, i, "v." i ",") ")
            }
        }
        "
        "impl<" type_params_static_bound "> " cm.TupleStructure "<" cm.TupleN_ident "<" type_params "> > for" tuple_owned "{
            #[coverage(off)]
            fn get_ref<'__fuzzcheck_derive_lt>(&'__fuzzcheck_derive_lt self) -> " tuple_ref " {
                (" join_ts!(0..nbr_elements, i, "&self." i ",") ")
            }
            #[coverage(off)]
            fn get_mut<'__fuzzcheck_derive_lt>(&'__fuzzcheck_derive_lt mut self) -> " tuple_mut " {
                (" join_ts!(0..nbr_elements, i, "&mut self." i ",") ")
            }
            #[coverage(off)]
            fn new(t: " tuple_owned ") -> Self {
                t
            }
        }"
    );
}

#[allow(non_snake_case)]
pub(crate) fn impl_tuple_structure_trait(
    tb: &mut TokenBuilder,
    struct_ident: &Ident,
    generics: &Generics,
    struc: &DataStruct,
) {
    let nbr_elements = struc.fields.len();
    let cm = Common::new(nbr_elements);
    let field_types = join_ts!(&struc.fields, field, field.ty, separator: ",");
    // let Ti = |i: usize| ident!("T" i);

    let TupleKind = cm.TupleN_path.clone();

    let tuple_owned = ts!("(" join_ts!(&struc.fields, field, field.ty ",") ")");
    let tuple_ref = ts!("(" join_ts!(&struc.fields, field, "&'__fuzzcheck_derive_lt" field.ty ",") ")");
    let tuple_mut = ts!("(" join_ts!(&struc.fields, field, "&'__fuzzcheck_derive_lt mut" field.ty ",") ")");

    let mut new_generics = generics.clone();
    if new_generics.where_clause.is_none() {
        new_generics.where_clause = Some(WhereClause {
            where_token: <_>::default(),
            predicates: <_>::default(),
        });
    }
    for tp in generics.type_params() {
        let where_clause = new_generics.where_clause.as_mut().unwrap();
        where_clause
            .predicates
            .push(parse2(ts!(tp.ident.clone() ": 'static")).unwrap());
    }
    let generics = new_generics;

    let generics_split = generics.split_for_impl();
    let selfty = ts!(struct_ident q!(generics_split.1));

    extend_ts!(tb,
        "impl" q!(generics_split.0) cm.TupleStructure "<" TupleKind "<" field_types "> >
            for" selfty q!(generics_split.2) "{
            #[coverage(off)]
            fn get_ref<'__fuzzcheck_derive_lt>(&'__fuzzcheck_derive_lt self) -> " tuple_ref " {
                (" join_ts!(struc.fields.iter().enumerate(), (idx, field), "&self." access_field(field, idx) ",") ")
            }

            #[coverage(off)]
            fn get_mut<'__fuzzcheck_derive_lt>(&'__fuzzcheck_derive_lt mut self) -> " tuple_mut " {
                (" join_ts!(struc.fields.iter().enumerate(), (idx, field), "&mut self." access_field(field, idx) ",") ")
            }

            #[coverage(off)]
            fn new(t:" tuple_owned ") -> Self {
                Self {"
                    join_ts!(struc.fields.iter().enumerate(), (idx, field),
                        access_field(field, idx) ": t." idx ","
                    )

                "}
            }
        }"
    );
}

pub(crate) fn impl_default_mutator_for_struct_with_0_field(
    tb: &mut TokenBuilder,
    struct_ident: &Ident,
    struc: &DataStruct,
) {
    assert!(struc.fields.is_empty());
    let cm = Common::new(0);

    let init = match struc.fields {
        syn::Fields::Named(_) => ts!("{}"),
        syn::Fields::Unnamed(_) => ts!("()"),
        syn::Fields::Unit => ts!(),
    };

    extend_ts!(tb, 
    "impl " cm.DefaultMutator "for" struct_ident "{
        type Mutator = " cm.UnitMutator "<Self>;
    
        #[coverage(off)]
        fn default_mutator() -> Self::Mutator {
            Self::Mutator::new(" struct_ident init ", 0.0)
        }
    }
    ");
}

#[allow(non_snake_case)]
pub(crate) fn impl_default_mutator_for_struct(
    tb: &mut TokenBuilder,
    struct_ident: &Ident,
    generics: &Generics,
    visibility: &Visibility,
    struc: &DataStruct,
    settings: &MakeMutatorSettings,
) {
    let nbr_elements = struc.fields.len();

    let cm = Common::new(nbr_elements);
    let TupleNMutator = cm.TupleNMutator.as_ref()(nbr_elements);

    let field_types = join_ts!(&struc.fields, field, field.ty, separator: ",");

    let field_mutators = vec![struc
        .fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
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
                    j: None,
                    field: field.clone(),
                    kind: FieldMutatorKind::Prescribed(m.0.clone(), m.1),
                }
            } else {
                FieldMutator {
                    i,
                    j: None,
                    field: field.clone(),
                    kind: FieldMutatorKind::Generic,
                }
            }
        })
        .collect::<Vec<_>>()];

    let TupleKind = cm.TupleN_path.clone();

    let TupleN_and_generics = ts!(TupleKind "<" field_types ">");

    let TupleMutatorWrapper = ts!(
        cm.TupleMutatorWrapper "<"
            TupleNMutator "<"
                join_ts!(field_mutators.iter().flatten(), m,
                    m.mutator_stream(&cm)
                , separator: ",")
            ">,"
            TupleN_and_generics
        ">"
    );

    use crate::structs_and_enums::{make_mutator_type_and_impl, CreateWrapperMutatorParams};

    let params = CreateWrapperMutatorParams {
        cm: &cm,
        visibility,
        type_ident: struct_ident,
        type_generics: generics,
        field_mutators: &field_mutators,
        InnerMutator: &TupleMutatorWrapper,
        new_impl: &ts!(
            "
            #[coverage(off)]
            pub fn new("
            join_ts!(struc.fields.iter().zip(field_mutators.iter().flatten()).enumerate(), (idx, (field, mutator)),
                ident!("mutator_" access_field(field, idx)) ":" mutator.mutator_stream(&cm)
            , separator: ",")
            ") -> Self {
            Self {
                mutator : " cm.TupleMutatorWrapper "::new(" TupleNMutator "::new("
                    join_ts!(struc.fields.iter().enumerate(), (idx, field),
                        ident!("mutator_" access_field(field, idx))
                    , separator: ",")
                    "))
            }
            }"
        ),
        settings,
    };

    extend_ts!(tb, make_mutator_type_and_impl(params));
}

#[allow(non_snake_case)]
fn declare_tuple_mutator(tb: &mut TokenBuilder, nbr_elements: usize) {
    let cm = Common::new(nbr_elements);

    let mutator_type_params = join_ts!(0..nbr_elements, i, ident!("M" i), separator: ",");
    let type_params = ts!(mutator_type_params);
    extend_ts!(tb,
        format!("/// A `TupleMutator` for types that have a {n}-tuple structure", n=nbr_elements)
        "#[derive(" cm.Default ")]"
        "pub struct" cm.TupleNMutator_ident "<" type_params ">"
        "{"
            join_ts!(0..nbr_elements, i,
                ident!("mutator_" i) ":" ident!("M" i) ","
            )
            "rng :" cm.fastrand_Rng ",
        }

        impl < " type_params " >" cm.TupleNMutator_ident "<" type_params "> {
            #[coverage(off)]
            pub fn new(" join_ts!(0..nbr_elements, i, ident!("mutator_" i) ":" ident!("M" i), separator: ",") ") -> Self {
                Self {"
                    join_ts!(0..nbr_elements, i,
                        ident!("mutator_" i) ","
                    )
                    "rng: <_>::default() ,
                    "
                "}
            }"
        "}"
    )
}

#[allow(non_snake_case)]
fn declare_tuple_mutator_helper_types(tb: &mut TokenBuilder, nbr_elements: usize) {
    let cm = Common::new(nbr_elements);
    let Ti = cm.Ti.as_ref();
    let ti = cm.ti.as_ref();
    let tuple_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");

    extend_ts!(tb,
        "
        #[doc(hidden)]
        #[derive(" cm.Clone ")]
        pub struct Cache <" tuple_type_params "> {"
            join_ts!(0..nbr_elements, i,
                ti(i) ":" Ti(i) ","
            )
            "cplx : f64,
            vose_alias : " cm.VoseAlias "
        }
        #[doc(hidden)]
        #[derive(" cm.Clone ")]
        pub enum TupleIndex {"
            join_ts!(0..nbr_elements, i,
                Ti(i)
            , separator: ",")
        "}
        #[doc(hidden)]
        #[derive(" cm.Clone ")]
        pub struct MutationStep < " tuple_type_params ", " join_ts!(0..nbr_elements, i, ident!("MS" i) ",")  " > {"
            join_ts!(0..nbr_elements, i,
                ti(i) ":" ident!("MS" i) ","
            )
            "inner : " cm.Vec " < TupleIndex > ,
            vose_alias : Option<" cm.VoseAlias ">,
            "
            join_ts!(0..nbr_elements, i,
                ident!("crossover_step_" i) ":" cm.CrossoverStep "<" Ti(i) ">,"
            )
            "
        }
        #[doc(hidden)]
        pub enum UnmutateElementToken<T, U> {
            Replace(T),
            Unmutate(U)
        }
        #[doc(hidden)]
        pub struct UnmutateToken < " tuple_type_params ", " join_ts!(0..nbr_elements, i, ident!("U" i), separator: ",") " > {"
            join_ts!(0..nbr_elements, i,
                "pub" ti(i) ": " cm.Option "<UnmutateElementToken<" Ti(i) "," ident!("U" i) ">>,"
            )
            "
        }
        impl < " tuple_type_params ", " join_ts!(0..nbr_elements, i, ident!("U" i), separator: ",") " > " cm.Default " for UnmutateToken < " tuple_type_params ", " join_ts!(0..nbr_elements, i, ident!("U" i), separator: ",") " >  {
            #[coverage(off)]
            fn default() -> Self {
                Self {"
                    join_ts!(0..nbr_elements, i,
                        ti(i) ":" cm.None ","
                    )
                    "
                }
            }
        }
        "
    )
}

#[allow(non_snake_case)]
fn impl_mutator_trait(tb: &mut TokenBuilder, nbr_elements: usize) {
    let cm = Common::new(nbr_elements);

    let tuple_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");

    let mutator_type_params = join_ts!(0..nbr_elements, i, ident!("M" i), separator: ",");
    let type_params = ts!(tuple_type_params "," mutator_type_params);

    let ti = cm.ti.as_ref();
    let Ti = cm.Ti.as_ref();
    let Mi = cm.Mi.as_ref();
    let mutator_i = cm.mutator_i.as_ref();
    let ti_value = cm.ti_value.as_ref();

    // let tuple_owned = ts!("(" join_ts!(0..nbr_elements, i, Ti(i), separator: ",") ")");

    let SelfAsTupleMutator = ts!("<Self as " cm.TupleMutator "<T, " cm.TupleN_ident "<" tuple_type_params "> >>");

    let TupleNAsRefTypes = ts!("<" cm.TupleN_ident "<" tuple_type_params "> as " cm.RefTypes ">");
    let tuple_ref = ts!(
       TupleNAsRefTypes "::Ref<'__fuzzcheck_derive_lt>"
    );
    let tuple_mut = ts!(
       TupleNAsRefTypes "::Mut<'__fuzzcheck_derive_lt>"
    );

    extend_ts!(tb,"
    impl <T , " type_params " > " cm.TupleMutator "<T , " cm.TupleN_ident "<" tuple_type_params "> > 
        for " cm.TupleNMutator_ident "< " mutator_type_params " >
    where
        T: " cm.Clone " + 'static," 
        join_ts!(0..nbr_elements, i,
            Ti(i) ":" cm.Clone " + 'static ,"
            Mi(i) ":" cm.fuzzcheck_traits_Mutator "<" Ti(i) ">,"
        ) "
        T: " cm.TupleStructure "<" cm.TupleN_ident "<" tuple_type_params "> >,
    {
        #[doc(hidden)]
        type Cache = Cache <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" cm.fuzzcheck_traits_Mutator "<" Ti(i) "> >::Cache "
            , separator: ",")
        ">;
        #[doc(hidden)]
        type MutationStep = MutationStep <"
            tuple_type_params ","
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" cm.fuzzcheck_traits_Mutator "<" Ti(i) "> >::MutationStep "
            , separator: ",")
        ">;

        #[doc(hidden)]
        type ArbitraryStep = ();

        #[doc(hidden)]
        type UnmutateToken = UnmutateToken <"
            tuple_type_params
            ","
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" cm.fuzzcheck_traits_Mutator "<" Ti(i) "> >::UnmutateToken "
            , separator: ",")
        ">;
        
        #[coverage(off)]
        fn initialize(&self) {"
            join_ts!(0..nbr_elements, i,
                "self." mutator_i(i) ".initialize();"
            )
        "}

        #[doc(hidden)]
        #[coverage(off)]
        fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn max_complexity(&self) -> f64 {"
            join_ts!(0..nbr_elements, i,
                "self." mutator_i(i) ".max_complexity()"
            , separator: "+")
        "}

        #[doc(hidden)]
        #[coverage(off)]
        fn global_search_space_complexity(&self) -> f64 {"
            join_ts!(0..nbr_elements, i,
                "self. " mutator_i(i) ".global_search_space_complexity()"
            , separator: "+") "
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn min_complexity(&self) -> f64 {"
            join_ts!(0..nbr_elements, i,
                "self." mutator_i(i) ".min_complexity()"
            , separator: "+")
        "}
        #[doc(hidden)]
        #[coverage(off)]
        fn complexity<'__fuzzcheck_derive_lt>(&self, _value: " tuple_ref ", cache: &'__fuzzcheck_derive_lt Self::Cache) -> f64 {
            cache.cplx
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn is_valid<'__fuzzcheck_derive_lt>(&self, value: " tuple_ref ") -> bool {"
             join_ts!(0..nbr_elements, i,
                "self." mutator_i(i) ".is_valid(value." i ")"
            , separator: "&&")
        "}

        #[doc(hidden)]
        #[coverage(off)]
        fn validate_value<'__fuzzcheck_derive_lt>(&self, value: " tuple_ref ") -> " cm.Option "<Self::Cache> {"
            join_ts!(0..nbr_elements, i,
                "let" ident!("c" i) " = self." mutator_i(i) ".validate_value(value." i ")?;"
            )
            join_ts!(0..nbr_elements, i,
                "let" ident!("cplx_" i) " = self." mutator_i(i) ".complexity(value." i ", &" ident!("c" i) ");"
            )

            "let sum_cplx = "
                join_ts!(0..nbr_elements, i,
                    ident!("cplx_" i)
                , separator: "+") ";

            let mut probabilities = vec!["
                join_ts!(0..nbr_elements, i,
                    "10. +" ident!("cplx_" i)
                , separator: ",") "
            ];
            let vose_alias = " cm.VoseAlias "::new(probabilities);

            let cache = Self::Cache {"
                join_ts!(0..nbr_elements, i, ti(i) ":" ident!("c" i) ",")
                "cplx: sum_cplx,
                vose_alias,
            };

            " cm.Some "(cache)
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn default_mutation_step<'__fuzzcheck_derive_lt>(&self, value: " tuple_ref ", cache: &'__fuzzcheck_derive_lt Self::Cache) -> Self::MutationStep {"
            join_ts!(0..nbr_elements, i,
                "let" ident!("s" i) " = self." mutator_i(i) ".default_mutation_step(value." i ", &cache. " ti(i) ");"
            )"
            "
            join_ts!(0 .. nbr_elements, i,
                "let" ident!("crossover_step_" i) "=" cm.CrossoverStep "::<" Ti(i) ">::default() ;"
            )
            "
            let all_indices = vec![" join_ts!(0..nbr_elements, i, "TupleIndex::" Ti(i), separator: ",") "];
             Self::MutationStep {"
                join_ts!(0..nbr_elements, i, ti(i) ":" ident!("s" i) ",")
                "inner: all_indices,
                vose_alias: Some(cache.vose_alias.clone()),
                "
                 join_ts!(0..nbr_elements, i,
                    ident!("crossover_step_" i) ","
                )
                "
            }
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn ordered_arbitrary(
            &self,
            step: &mut Self::ArbitraryStep,
            max_cplx: f64,
        ) -> " cm.Option "<(T, f64)> {
            if max_cplx < <Self as" cm.TupleMutator "<T , " cm.TupleN_ident "<" tuple_type_params "> > >::min_complexity(self) { 
                return " cm.None " 
            }
            " // TODO: actually write something that is ordered_arbitrary sense here
            cm.Some "  (self.random_arbitrary(max_cplx))
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {"
            join_ts!(0..nbr_elements, i,
                "let mut" ti_value(i) ":" cm.Option "<_> =" cm.None ";"
            )
            "let mut indices = ( 0 .." q!(nbr_elements) ").collect::<" cm.Vec "<_>>();"
            "self.rng.shuffle(&mut indices);"
            "let mut sum_cplx = 0.0;
            for idx in indices.iter() {
                match idx {"
                join_ts!(0..nbr_elements, i,
                    i "=> {
                        let (value, cplx) = self." mutator_i(i) ".random_arbitrary(max_cplx - sum_cplx);
                        " ti_value(i) " = Some(value);
                        sum_cplx += cplx;
                    }"
                )
                    "_ => unreachable!() ,
                }
            }
            (
                T::new(
                    ("
                    join_ts!(0..nbr_elements, i,
                        ti_value(i) ".unwrap(),"
                    )
                    ")
                ),
                sum_cplx,
            )
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn ordered_mutate<'__fuzzcheck_derive_lt>(
            &self,
            value: " tuple_mut ",
            cache: &'__fuzzcheck_derive_lt mut Self::Cache,
            step: &'__fuzzcheck_derive_lt mut Self::MutationStep,
            subvalue_provider: &dyn " cm.SubValueProvider ",
            max_cplx: f64,
        ) -> " cm.Option "<(Self::UnmutateToken, f64)> {
            if self.rng.u8(.. fuzzcheck::CROSSOVER_RATE ) == 0 {
                let current_cplx = " SelfAsTupleMutator "::complexity(self, " TupleNAsRefTypes "::get_ref_from_mut(&value), cache); 

                let idx = self.rng.usize(.. " q!(nbr_elements) ");
                match idx {
                    "
                    join_ts!(0 .. nbr_elements, i,
                        i "=> {
                            let old_field_cplx = self." mutator_i(i) ".complexity(value." i ", &cache." ti(i) ");
                            let max_field_cplx = max_cplx - current_cplx + old_field_cplx;
                            if let " cm.Some " ((subvalue, new_field_cplx)) = step." ident!("crossover_step_" i) ".get_next_subvalue(subvalue_provider, max_field_cplx) {
                                if self." mutator_i(i) ".is_valid(subvalue) {
                                    let mut replacer = subvalue.clone();
                                    ::std::mem::swap(value." i ", &mut replacer);
                                    let mut token = Self::UnmutateToken::default();
                                    return " cm.Some "((Self::UnmutateToken {
                                            " ti(i) ": " cm.Some "(UnmutateElementToken::Replace(replacer)),
                                            ..Self::UnmutateToken::default()
                                        }, current_cplx - old_field_cplx + new_field_cplx
                                    ));
                                }
                            }
                        }"
                    )
                    "_ => unreachable!()"
                    "
                }
            }
            if max_cplx < <Self as" cm.TupleMutator "<T , " cm.TupleN_ident "<" tuple_type_params "> > >::min_complexity(self) { return " cm.None " }
            if step.inner.is_empty() || step.vose_alias.is_none() {
                let idx1 = self.rng.usize(.." q!(nbr_elements) ");
                let mut idx2 = self.rng.usize(.." q!(nbr_elements) " - 1);
                if idx2 >= idx1 {
                    idx2 += 1;
                }
                assert!(idx1 != idx2);
                let mut whole_token = Self::UnmutateToken::default();
                let mut current_cplx = " SelfAsTupleMutator "::complexity(self, " TupleNAsRefTypes "::get_ref_from_mut(&value), cache); 
                for idx in [idx1, idx2] {
                    match idx {"
                    join_ts!(0..nbr_elements, i,
                        i "=> {
                            let old_field_cplx = self." mutator_i(i) ".complexity(value." i ", &cache." ti(i) ");
                            let max_field_cplx = max_cplx - current_cplx + old_field_cplx;
                            let (token, new_field_cplx) = self." mutator_i(i) "
                                .random_mutate(value." i ", &mut cache." ti(i) ", max_field_cplx) ;
                            whole_token. " ti(i) " = " cm.Some "(UnmutateElementToken::Unmutate(token));
                            current_cplx = current_cplx - old_field_cplx + new_field_cplx;
                        }"
                    )
                    "_ => unreachable!()"
                    "}
                }
                return " cm.Some "( (whole_token, current_cplx) );
            }
            let vose_alias = step.vose_alias.as_ref().unwrap();
            let step_idx = vose_alias.sample();

            let current_cplx = " SelfAsTupleMutator "::complexity(self, " TupleNAsRefTypes "::get_ref_from_mut(&value), cache); 
            let inner_step_to_remove: usize;

            match step.inner[step_idx] {"
            join_ts!(0..nbr_elements, i,
                "TupleIndex::" Ti(i) "=> {
                    let old_field_cplx = self." mutator_i(i) ".complexity(value." i ", &cache." ti(i) ");
                    let max_field_cplx = max_cplx - current_cplx + old_field_cplx;
                    if let " cm.Some "((token, new_field_cplx)) =
                        self." mutator_i(i) "
                            .ordered_mutate(value." i ", &mut cache." ti(i) ", &mut step." ti(i) ", subvalue_provider, max_field_cplx)
                    {
                        return " cm.Some "((Self::UnmutateToken {
                            " ti(i) ": " cm.Some "(UnmutateElementToken::Unmutate(token)),
                            ..Self::UnmutateToken::default()
                        }, current_cplx - old_field_cplx + new_field_cplx));
                    } else {
                        inner_step_to_remove = step_idx;
                    }
                }"
            )"
            }
            let mut prob = vose_alias.original_probabilities.clone();
            prob[inner_step_to_remove] = 0.0;
            let sum = prob.iter().sum::<f64>();
            if sum == 0.0 {
                step.vose_alias = " cm.None ";
            }  else {
                step.vose_alias = " cm.Some "(" cm.VoseAlias "::new(prob));
            }
            " SelfAsTupleMutator "::ordered_mutate(self, value, cache, step, subvalue_provider, max_cplx)
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn random_mutate<'__fuzzcheck_derive_lt>(&self, value: " tuple_mut ", cache: &'__fuzzcheck_derive_lt mut Self::Cache, max_cplx: f64, ) -> (Self::UnmutateToken, f64) {
            let current_cplx = " SelfAsTupleMutator "::complexity(self, " TupleNAsRefTypes "::get_ref_from_mut(&value), cache);
            match cache.vose_alias.sample() {"
                join_ts!(0..nbr_elements, i,
                    i "=> {
                        let old_field_cplx = self." mutator_i(i) ".complexity(value." i ", &cache." ti(i) ");
                        let max_field_cplx = max_cplx - current_cplx + old_field_cplx;
                        let (token, new_field_cplx) = self." mutator_i(i) "
                            .random_mutate(value." i ", &mut cache." ti(i) ", max_field_cplx) ;
                        
                        return (Self::UnmutateToken {
                            " ti(i) ": " cm.Some "(UnmutateElementToken::Unmutate(token)),
                            ..Self::UnmutateToken::default()
                        },  current_cplx - old_field_cplx + new_field_cplx);
                    }"
                )
                "_ => unreachable!() ,
            }
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn unmutate<'__fuzzcheck_derive_lt>(&'__fuzzcheck_derive_lt self, value: " tuple_mut ", cache: &'__fuzzcheck_derive_lt mut Self::Cache, t: Self::UnmutateToken) {"
            join_ts!(0..nbr_elements, i,
                "if let" cm.Some "(element_token) = t." ti(i) "{
                    match element_token {
                        UnmutateElementToken::Unmutate(subtoken) => {
                            self. " mutator_i(i) ".unmutate(value." i ", &mut cache." ti(i) ", subtoken);
                        }
                        UnmutateElementToken::Replace(e) => {
                            *value." i " = e;
                        }
                    }
                }"
            )
        "}

        #[doc(hidden)]
        #[coverage(off)]
        fn visit_subvalues<'__fuzzcheck_derive_lt>(&self, value: " tuple_ref ", cache: &'__fuzzcheck_derive_lt Self::Cache, visit: &mut dyn FnMut(&'__fuzzcheck_derive_lt dyn" cm.Any ", f64)) {"
            join_ts!(0..nbr_elements, i,
                "
                let cplx = self. " mutator_i(i) ".complexity(value. " i ", &cache. " ti(i) "); 
                visit(value." i ", cplx);
                self." mutator_i(i) ".visit_subvalues(value." i ", &cache. " ti(i) ", visit);
                "
            )
            "
        }
    }"
    )
}

#[allow(non_snake_case)]
fn impl_default_mutator_for_tuple(tb: &mut TokenBuilder, nbr_elements: usize) {
    let cm = Common::new(nbr_elements);

    let Ti = cm.Ti.as_ref();

    let tuple_type_params = join_ts!(0..nbr_elements, i, Ti(i), separator: ",");

    let TupleN = ts!(ident!("Tuple" nbr_elements) "<" tuple_type_params ">");
    let TupleMutatorWrapper = ts!(
        cm.TupleMutatorWrapper "<"
            cm.TupleNMutator_ident "<"
                join_ts!(0..nbr_elements, i,
                    "<" Ti(i) "as" cm.DefaultMutator "> :: Mutator"
                , separator: ",")
            ">,"
            TupleN
        ">"
    );

    extend_ts!(tb,
    // "
    // impl<" type_params ">" cm.Default "for" cm.TupleNMutator_ident "<" mutator_type_params ">
    //     where"
    //     join_ts!(0..nbr_elements, i, Mi(i) ":" cm.Default, separator: ",")
    // "{
    //     fn default() -> Self {
    //         Self::new("
    //             join_ts!(0..nbr_elements, i,
    //                 "<" Mi(i) "as" cm.Default "> :: default()"
    //             , separator: ",")
    //         ")
    //     }
    // }
    "
    impl<" tuple_type_params ">" cm.DefaultMutator "for (" tuple_type_params ",)
        where" join_ts!(0..nbr_elements, i, Ti(i) ":" cm.DefaultMutator "+ 'static", separator: ",")
    "{
        type Mutator = " TupleMutatorWrapper ";
        #[coverage(off)]
        fn default_mutator() -> Self::Mutator {
            Self::Mutator::new(" cm.TupleNMutator_ident "::new("
                join_ts!(0..nbr_elements, i,
                    "<" Ti(i) "as" cm.DefaultMutator "> :: default_mutator()"
                , separator: ",")
            "))
        }
    }"
    )
}
