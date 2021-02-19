use decent_synquote_alternative as synquote;
use proc_macro2::{Ident, Span, TokenStream};

use synquote::parser::*;
use synquote::token_builder::*;

use crate::{Common, MakeMutatorSettings};

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
    let tuple_owned = ts!("(" type_params ")");
    let tuple_ref = ts!("(" join_ts!(0..nbr_elements, i, "&'a" Ti(i), separator: ",") ")");
    let tuple_mut = ts!("(" join_ts!(0..nbr_elements, i, "&'a mut" Ti(i), separator: "," ) ")");

    let PhantomData = ts!(cm.PhantomData "<(" type_params ")>");

    extend_ts!(tb,
        "pub struct" cm.TupleN_ident "<" type_params_static_bound "> {
            _phantom: " PhantomData ",
        }
        impl<" type_params_static_bound "> " cm.RefTypes " for " cm.TupleN_ident "<" type_params "> {
            type Owned = " tuple_owned ";
            type Ref<'a> = " tuple_ref ";
            type Mut<'a> = " tuple_mut ";
            fn get_ref_from_mut<'a>(v: &'a Self::Mut<'a>) -> Self::Ref<'a> {
                (" join_ts!(0..nbr_elements, i, "v." i, separator: ",") ")
            }
        }
        "
        "impl<" type_params_static_bound "> " cm.TupleStructure "<" cm.TupleN_ident "<" type_params "> > for" tuple_owned "{
            fn get_ref<'a>(&'a self) -> " tuple_ref " {
                (" join_ts!(0..nbr_elements, i, "&self." i, separator: ",") ")
            }

            fn get_mut<'a>(&'a mut self) -> " tuple_mut " {
                (" join_ts!(0..nbr_elements, i, "&mut self." i, separator: ",") ")
            }
            fn new(t: " tuple_owned ") -> Self {
                t
            }
        }"
    );
}

#[allow(non_snake_case)]
pub(crate) fn impl_tuple_structure_trait(tb: &mut TokenBuilder, struc: &Struct) {
    let nbr_elements = struc.struct_fields.len();
    let cm = Common::new(nbr_elements);
    let field_types = join_ts!(&struc.struct_fields, field, field.ty, separator: ",");
    // let Ti = |i: usize| ident!("T" i);

    let TupleKind = if nbr_elements == 1 {
        cm.Wrapped_path.clone()
    } else {
        cm.TupleN_path.clone()
    };

    let generics_no_eq = struc.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = struc.generics.removing_bounds_and_eq_type();

    let tuple_owned = ts!("(" join_ts!(&struc.struct_fields, field, field.ty , separator: ",") ")");
    let tuple_ref = ts!("(" join_ts!(&struc.struct_fields, field, "&'a" field.ty , separator: ",") ")");
    let tuple_mut = ts!("(" join_ts!(&struc.struct_fields, field, "&'a mut" field.ty , separator: ",") ")");

    let mut where_clause = struc.where_clause.clone().unwrap_or_default();
    where_clause.add_clause_items(
        join_ts!(&struc.generics.type_params, tp,
            tp.type_ident ": 'static,"
        )
    );

    extend_ts!(tb,
        "impl" generics_no_eq cm.TupleStructure "<" TupleKind "<" field_types "> >
            for" struc.ident generics_no_eq_nor_bounds where_clause "{
            fn get_ref<'a>(&'a self) -> " tuple_ref " {
                (" join_ts!(&struc.struct_fields, field, "&self." field.access(), separator: ",") ")
            }

            fn get_mut<'a>(&'a mut self) -> " tuple_mut " {
                (" join_ts!(&struc.struct_fields, field, "&mut self." field.access(), separator: ",") ")
            }

            fn new(t:" tuple_owned ") -> Self {
                Self {"
                    if nbr_elements > 1 {
                        join_ts!(struc.struct_fields.iter().enumerate(), (i, field),
                            field.access() ": t." i
                        , separator: ",")
                    } else {
                        ts!(struc.struct_fields[0].access() ": t")
                    }
                "}
            }
        }"
    );
}

pub(crate) fn impl_default_mutator_for_struct_with_0_field(
    tb: &mut TokenBuilder,
    struc: &Struct
) {
    assert!(struc.struct_fields.len() == 0);
    let cm = Common::new(0);
    let generics_no_eq = struc.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = struc.generics.removing_bounds_and_eq_type();

    // add T: DefaultMutator for each generic type parameter to the existing where clause
    let mut where_clause = struc.where_clause.clone().unwrap_or(WhereClause::default());
    where_clause.add_clause_items(
        join_ts!(&struc.generics.type_params, ty_param,
            ty_param ":" cm.DefaultMutator ","
        )
    );

    let init = struc.kind.map(|kind| ts!(kind.open() kind.close()));

    extend_ts!(tb, 
    "impl " generics_no_eq cm.DefaultMutator "for" struc.ident generics_no_eq_nor_bounds where_clause "{
        type Mutator = " cm.UnitMutator "<Self>;
    
        fn default_mutator() -> Self::Mutator {
            Self::Mutator::new(" struc.ident init ")
        }
    }
    ");
}

enum FieldMutator {
    Generic(usize, StructField),
    Prescribed(Ty, Option<TokenStream>),
}

#[allow(non_snake_case)]
pub(crate) fn impl_default_mutator_for_struct_with_more_than_1_field(
    tb: &mut TokenBuilder,
    struc: &Struct,
    settings: &MakeMutatorSettings,
) {
    let nbr_elements = struc.struct_fields.len();

    let cm = Common::new(nbr_elements);
    let Mi = cm.Mi.as_ref();
    let TupleNMutator = cm.TupleNMutator.as_ref()(nbr_elements);

    let generics_no_eq = struc.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = struc.generics.removing_bounds_and_eq_type();

    let field_types = join_ts!(&struc.struct_fields, field, field.ty, separator: ",");

    let field_mutators = struc.struct_fields.iter().enumerate().map(|(i, field)| {
        let mut mutator = None;
        for attribute in field.attributes.iter() {
            if let Some((m, init)) = super::read_field_default_mutator_attribute(attribute.clone()) {
                mutator = Some((m, init));
            }
        }
        if let Some(m) = mutator {
            FieldMutator::Prescribed(m.0.clone(), m.1.clone())
        } else {
            FieldMutator::Generic(i, field.clone())
        }
    }).collect::<Vec<_>>();

    let field_generic_mutators = field_mutators.iter().filter_map(|m| {
        match m {
            FieldMutator::Generic(i, field) => { Some((*i, field.clone())) }
            FieldMutator::Prescribed(_, _) => { None }
        }
    }).collect::<Vec<_>>();
    let field_prescribed_mutators = field_mutators.iter().filter_map(|m| {
        match m {
            FieldMutator::Generic(_, _) => { None }
            FieldMutator::Prescribed(m, init) => { Some((m.clone(), init.clone())) }
        }
    }).collect::<Vec<_>>();

    let field_mutators_streams = field_mutators.iter().map(|m| {
        match m {
            FieldMutator::Prescribed(m, _) => {
                ts!(m)
            }
            FieldMutator::Generic(i, _) => {
                ts!(Mi(*i))
            }
        }
    }).collect::<Vec<_>>();

    let TupleKind = if nbr_elements == 1 {
        cm.Wrapped_path.clone()
    } else {
        cm.TupleN_path.clone()
    };

    let TupleN_and_generics = ts!(TupleKind "<" field_types ">");

    let StrucMutator = if let Some(name) = &settings.name {
        name.clone()
    } else {
        ident!(struc.ident "Mutator")
    };

    let TupleMutatorWrapper = ts!(
        cm.TupleMutatorWrapper "<"
            struc.ident generics_no_eq_nor_bounds ","
            TupleNMutator "<"
                field_types ", "
                join_ts!(&field_mutators_streams, m, 
                    m
                , separator: ",")
            ">,"
            TupleN_and_generics
        ">"
    );

    let mut StrucMutator_where_clause = struc.where_clause.clone().unwrap_or(WhereClause::default());
    StrucMutator_where_clause.add_clause_items(ts!(
        // add T: Clone + 'static for each generic type parameter to the existing where clause
        join_ts!(&struc.generics.type_params, ty_param,
            ty_param.type_ident ":" cm.Clone "+ 'static,"
        )
        // and then add Mi : Mutator<field> for each generic field mutator
        join_ts!(field_generic_mutators.iter(), (i, field),
            Mi(*i) ":" cm.fuzzcheck_mutator_traits_Mutator "<" field.ty "> ,"
        )
    ));

    let mut StrucMutator_generics = struc.generics.clone();
    for (i, _) in field_generic_mutators.iter() {
        StrucMutator_generics.type_params.push(TypeParam {
            type_ident: ts!(Mi(*i)),
            ..<_>::default()
        });
    }

    // // add T: DefaultMutator + 'static for each generic type parameter to the existing where clause
    let mut DefaultMutator_where_clause = struc.where_clause.clone().unwrap_or(WhereClause::default());
    DefaultMutator_where_clause.add_clause_items(ts!(
        join_ts!(&struc.generics.type_params, ty_param,
            ty_param.type_ident ":" cm.DefaultMutator "+ 'static ,"
        )
        join_ts!(field_prescribed_mutators.iter().filter(|(_, init)| init.is_none()), (mutator, _),
            mutator ":" cm.Default ","
        )
    ));

    let mut DefaultMutator_Mutator_generics = struc.generics.removing_bounds_and_eq_type();
    for (_, field) in field_generic_mutators.iter() {
        DefaultMutator_Mutator_generics.type_params.push(TypeParam {
            type_ident: ts!("<" field.ty " as " cm.DefaultMutator ">::Mutator"),
            ..<_>::default()
        })
    }

    let StrucMutatorCache = ident!(StrucMutator "Cache");
    let StrucMutatorMutationStep = ident!(StrucMutator "MutationStep");
    let StrucMutatorArbitraryStep = ident!(StrucMutator "ArbitraryStep");
    let StrucMutatorUnmutateToken = ident!(StrucMutator "UnmutateToken");

    let helper_type = |helper_type: &str| {
        ts!(
            struc.visibility "struct" ident!(StrucMutator helper_type) StrucMutator_generics.removing_eq_type() StrucMutator_where_clause "{
            inner : "
                if settings.recursive {
                    ts!(cm.Box "<")
                } else {
                    ts!("")
                }
                "<" TupleMutatorWrapper " as " cm.fuzzcheck_mutator_traits_Mutator "<" struc.ident generics_no_eq_nor_bounds "> >::" helper_type
                if settings.recursive {
                    ">"
                } else {
                    ""
                }
                ",
            }
            impl " StrucMutator_generics.removing_eq_type() ident!(StrucMutator helper_type) StrucMutator_generics.removing_bounds_and_eq_type() StrucMutator_where_clause "{
                fn new(inner: <" TupleMutatorWrapper " as " cm.fuzzcheck_mutator_traits_Mutator "<" struc.ident generics_no_eq_nor_bounds "> >::" helper_type") -> Self {"
                    "Self {
                        inner: "  if settings.recursive { ts!(cm.Box "::new") } else { ts!("") }
                            "(inner)"
                        "
                    }"
                "}
            } 
            ")
    };
    let impl_clone_helper_type = |helper_type: &str| {
        ts!(
            "impl" StrucMutator_generics.removing_eq_type()  cm.Clone "for" ident!(StrucMutator helper_type) StrucMutator_generics.removing_bounds_and_eq_type() StrucMutator_where_clause "{
                fn clone(&self) -> Self {
                    Self {
                        inner: self.inner.clone()
                    }
                }
            }" 
        )
    };
    let impl_default_helper_type = |helper_type: &str| {
        ts!(
            "impl" StrucMutator_generics.removing_eq_type()  cm.Default "for" ident!(StrucMutator helper_type) StrucMutator_generics.removing_bounds_and_eq_type() StrucMutator_where_clause "{
                fn default() -> Self {
                    Self {
                        inner: <_>::default()
                    }
                }
            }" 
        )
    };

    extend_ts!(tb,
    struc.visibility "struct" StrucMutator StrucMutator_generics.removing_eq_type() StrucMutator_where_clause "{
        pub mutator: " TupleMutatorWrapper "
    }"
    helper_type("Cache")
    impl_clone_helper_type("Cache")
    helper_type("MutationStep")
    impl_clone_helper_type("MutationStep")
    helper_type("ArbitraryStep")
    impl_clone_helper_type("ArbitraryStep")
    impl_default_helper_type("ArbitraryStep")
    helper_type("UnmutateToken")
    "impl " StrucMutator_generics.removing_eq_type() StrucMutator StrucMutator_generics.removing_bounds_and_eq_type() StrucMutator_where_clause "{
        pub fn new(" 
            join_ts!(struc.struct_fields.iter().zip(field_mutators_streams.iter()), (field, mutator),
                ident!("mutator_" field.access()) ":" mutator
            , separator: ",")
            ") -> Self {
            Self {
                mutator : " cm.TupleMutatorWrapper "::new(" TupleNMutator "::new("
                    join_ts!(struc.struct_fields.iter(), field,
                        ident!("mutator_" field.access())
                    , separator: ",")
                    "))
            }
        }
    } "
    // TODO: should use the `init` of prescribed mutators
    "impl " StrucMutator_generics.removing_eq_type() cm.Default "for" StrucMutator StrucMutator_generics.removing_bounds_and_eq_type() 
        StrucMutator_where_clause ", " TupleMutatorWrapper ":" cm.Default "
    {
        fn default() -> Self {
            Self {
                mutator: <_>::default()
            }
        }
    }
    impl " StrucMutator_generics.removing_eq_type() cm.fuzzcheck_mutator_traits_Mutator "<" struc.ident generics_no_eq_nor_bounds ">"
        "for" StrucMutator StrucMutator_generics.removing_bounds_and_eq_type()
        StrucMutator_where_clause
    "{
        type Cache = " StrucMutatorCache StrucMutator_generics.removing_bounds_and_eq_type() ";
        type ArbitraryStep = " StrucMutatorArbitraryStep StrucMutator_generics.removing_bounds_and_eq_type() ";
        type MutationStep = " StrucMutatorMutationStep StrucMutator_generics.removing_bounds_and_eq_type() ";
        type UnmutateToken = " StrucMutatorUnmutateToken StrucMutator_generics.removing_bounds_and_eq_type() ";
        
        fn cache_from_value(&self, value: &" struc.ident generics_no_eq_nor_bounds ") -> Self::Cache {
            Self::Cache::new(self.mutator.cache_from_value(value))
        }

        fn initial_step_from_value(&self, value: &" struc.ident generics_no_eq_nor_bounds ") -> Self::MutationStep {
            Self::MutationStep::new(self.mutator.initial_step_from_value(value))
        }

        fn max_complexity(&self) -> f64 {
            self.mutator.max_complexity()
        }

        fn min_complexity(&self) -> f64 {
            self.mutator.min_complexity()
        }

        fn complexity(&self, value: &" struc.ident generics_no_eq_nor_bounds ", cache: &Self::Cache) -> f64 {
            self.mutator.complexity(value, &cache.inner)
        }

        fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(" struc.ident generics_no_eq_nor_bounds ", Self::Cache)> {
            if let " cm.Some "((value, cache)) = self.mutator.ordered_arbitrary(&mut step.inner, max_cplx) {"
                cm.Some "((value, Self::Cache::new(cache)))"
            "} else {"
                cm.None
            "}
        }

        fn random_arbitrary(&self, max_cplx: f64) -> (" struc.ident generics_no_eq_nor_bounds ", Self::Cache) {
            let (value, cache) = self.mutator.random_arbitrary(max_cplx);
            (value, Self::Cache::new(cache))
        }

        fn ordered_mutate(
            &self,
            value: &mut " struc.ident generics_no_eq_nor_bounds ",
            cache: &mut Self::Cache,
            step: &mut Self::MutationStep,
            max_cplx: f64,
        ) -> Option<Self::UnmutateToken> {
            if let " cm.Some "(t) = self.mutator.ordered_mutate(value, &mut cache.inner, &mut step.inner, max_cplx) {
                " cm.Some "(Self::UnmutateToken::new(t))
            } else {"
                cm.None   
            "}
        }

        fn random_mutate(&self, value: &mut " struc.ident generics_no_eq_nor_bounds ", cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
            Self::UnmutateToken::new(self.mutator.random_mutate(value, &mut cache.inner, max_cplx))
        }

        fn unmutate(&self, value: &mut " struc.ident generics_no_eq_nor_bounds ", cache: &mut Self::Cache, t: Self::UnmutateToken) {
            self.mutator.unmutate(
                value, 
                &mut cache.inner, "
                if settings.recursive {
                    "*t.inner"
                } else {
                    "t.inner"
                }
                "
            )
        }
    }
    "
    if settings.default {
        ts!("impl" generics_no_eq cm.DefaultMutator "for" struc.ident generics_no_eq_nor_bounds DefaultMutator_where_clause "{"
        if settings.recursive {
            ts!("type Mutator = " cm.RecursiveMutator "<" StrucMutator DefaultMutator_Mutator_generics ">;")
        } else {
            ts!("type Mutator = "  StrucMutator DefaultMutator_Mutator_generics ";")
        }
        "fn default_mutator() -> Self::Mutator {"
            if settings.recursive { 
                format!("{}::new(|self_| {{", cm.RecursiveMutator)
            } else { 
                "".to_string()
            }
            StrucMutator "::new("
                join_ts!(&field_mutators, mutator,
                    match mutator {
                     FieldMutator::Generic(_, field) => {
                        ts!("< " field.ty " >::default_mutator() ,")
                     }
                     FieldMutator::Prescribed(_, Some(init)) => {
                        ts!(init ",")
                     }
                     FieldMutator::Prescribed(mutator, None) => {
                        ts!("<" mutator ">::default() ,")
                     }
                    }
                )
            ")"
            if settings.recursive { 
                "})" 
            } else { 
                "" 
            }    
            "}
        }")
    } else {
        ts!()
    }
    )
}

#[allow(non_snake_case)]
fn declare_tuple_mutator(tb: &mut TokenBuilder, nbr_elements: usize) {
    let cm = Common::new(nbr_elements);
    let Ti = cm.Ti.as_ref();
    let Mi = cm.Mi.as_ref();

    let tuple_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");
    let mutator_type_params = join_ts!(0..nbr_elements, i, ident!("M" i), separator: ",");
    let type_params = ts!(tuple_type_params "," mutator_type_params);
    let tuple_type = ts!("(" tuple_type_params ")");

    let where_clause = ts!(
        "where"
        join_ts!(0..nbr_elements, i,
            Ti(i) ":" cm.Clone ","
            Mi(i) ":" cm.fuzzcheck_traits_Mutator "<" Ti(i) ">"
        ,separator: ",")
    );

    let mutator_type_params_replacing_one_by_m = |replacing: usize| -> TokenStream {
        join_ts!(0..nbr_elements, i, 
            if i == replacing {
                ident!("M")
            } else {
                Mi(i)
            }
        , separator: ",")
    };

    extend_ts!(tb,
        "pub struct" cm.TupleNMutator_ident "<" type_params ">" where_clause
        "{"
            join_ts!(0..nbr_elements, i,
                "pub" ident!("mutator_" i) ":" ident!("M" i) ","
            )
            "rng :" cm.fastrand_Rng ",
            _phantom :" cm.PhantomData "<" tuple_type ">"
        "}
        
        impl < " type_params " >" cm.TupleNMutator_ident "<" type_params ">" where_clause "{
            pub fn new(" join_ts!(0..nbr_elements, i, ident!("mutator_" i) ":" ident!("M" i), separator: ",") ") -> Self {
                Self {"
                    join_ts!(0..nbr_elements, i,
                        ident!("mutator_" i) ","
                    )
                    "rng: <_>::default() ,
                    _phantom:" cm.PhantomData
                "}
            }"
            join_ts!(0..nbr_elements, i,
                "pub fn" ident!("replacing_mutator_" i) " < M > ( self , mutator : M )
                    ->" cm.TupleNMutator_ident "<" tuple_type_params ", " mutator_type_params_replacing_one_by_m(i) " >" "
                    where M :" cm.fuzzcheck_traits_Mutator "<" ident!("T" i) "> 
                {
                    " cm.TupleNMutator_ident " {"
                        join_ts!(0..nbr_elements, j,
                            ident!("mutator_" j) ":" if i == j { ts!("mutator") } else { ts!("self ." ident!("mutator_" j)) } ","
                        )
                        "rng : self.rng ,
                        _phantom : self._phantom
                    }
                }"
            )
        "}"
    )
}

#[allow(non_snake_case)]
fn declare_tuple_mutator_helper_types(
    tb: &mut TokenBuilder,
    nbr_elements: usize,
) {
    let cm = Common::new(nbr_elements);
    let Ti = cm.Ti.as_ref();
    let ti = cm.ti.as_ref();
    let tuple_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");

    extend_ts!(tb,
        "#[derive( " cm.Clone " )]
        pub struct Cache <" tuple_type_params "> {"
            join_ts!(0..nbr_elements, i,
                ti(i) ":" ident!("T" i) ","
            )
            "cplx : f64
        }
        #[derive( " cm.Clone " )]
        pub enum InnerMutationStep {"
            join_ts!(0..nbr_elements, i,
                Ti(i)
            , separator: ",")
        "}
        #[derive( " cm.Clone " )]
        pub struct MutationStep < " tuple_type_params " > {"
            join_ts!(0..nbr_elements, i,
                ti(i) ":" Ti(i) ","
            )
            "step : usize ,
            inner : " cm.Vec " < InnerMutationStep > 
        }
        #[derive(" cm.Default "," cm.Clone ")]
        pub struct ArbitraryStep < " tuple_type_params " > {"
            join_ts!(0..nbr_elements, i,
                ti(i) ":" Ti(i)
            , separator: ",")
        "}

        pub struct UnmutateToken < " tuple_type_params " > {"
            join_ts!(0..nbr_elements, i,
                ti(i) ":" cm.Option "<" Ti(i) "> ,"
            )
            "cplx : f64
        }
        impl < " tuple_type_params " > " cm.Default " for UnmutateToken < " tuple_type_params " > {
            fn default() -> Self {
                Self {"
                    join_ts!(0..nbr_elements, i,
                        ti(i) ":" cm.None ","
                    )
                    "cplx : <_>::default()
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
    let ti_cache = cm.ti_cache.as_ref();

    // let tuple_owned = ts!("(" join_ts!(0..nbr_elements, i, Ti(i), separator: ",") ")");
    let tuple_ref = ts!("(" join_ts!(0..nbr_elements, i, "&'a" Ti(i), separator: ",") ")");
    let tuple_mut = ts!("(" join_ts!(0..nbr_elements, i, "&'a mut" Ti(i), separator: ",") ")");

    let SelfAsTupleMutator = ts!("<Self as " cm.TupleMutator "<T, " cm.TupleN_ident "<" tuple_type_params "> >>");
    let RefTypes = ts!(cm.fuzzcheck_mutators "::RefTypes");
    let TupleNAsRefTypes = ts!("<" cm.TupleN_ident "<" tuple_type_params "> as " RefTypes ">");

    extend_ts!(tb,"
    impl <T , " type_params " > " cm.TupleMutator "<T , " cm.TupleN_ident "<" tuple_type_params "> > 
        for " cm.TupleNMutator_ident "< " type_params " >
    where
        T: " cm.Clone "," 
        join_ts!(0..nbr_elements, i,
            Ti(i) ":" cm.Clone " + 'static ,"
            Mi(i) ":" cm.fuzzcheck_traits_Mutator "<" Ti(i) ">,"
        ) "
        T: " cm.TupleStructure "<" cm.TupleN_ident "<" tuple_type_params "> >,
    {
        type Cache = Cache <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" cm.fuzzcheck_traits_Mutator "<" Ti(i) "> >::Cache "
            , separator: ",")
        ">;
        type MutationStep = MutationStep <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" cm.fuzzcheck_traits_Mutator "<" Ti(i) "> >::MutationStep "
            , separator: ",")
        ">;
        type ArbitraryStep = ArbitraryStep <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" cm.fuzzcheck_traits_Mutator "<" Ti(i) "> >::ArbitraryStep "
            , separator: ",")
        ">;
        type UnmutateToken = UnmutateToken <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" cm.fuzzcheck_traits_Mutator "<" Ti(i) "> >::UnmutateToken "
            , separator: ",")
        ">;

        fn max_complexity(&self) -> f64 {"
            join_ts!(0..nbr_elements, i,
                "self." mutator_i(i) ".max_complexity()"
            , separator: "+")
        "}
        fn min_complexity(&self) -> f64 {"
            join_ts!(0..nbr_elements, i,
                "self." mutator_i(i) ".min_complexity()"
            , separator: "+")
        "}
        fn complexity<'a>(&'a self, _value: " tuple_ref ", cache: &'a Self::Cache) -> f64 {
            cache.cplx
        }
        fn cache_from_value<'a>(&'a self, value: " tuple_ref ") -> Self::Cache {"
            join_ts!(0..nbr_elements, i,
                "let" ti(i) "= self." mutator_i(i) ".cache_from_value(value." i ");"
            )
            "let cplx = "
                join_ts!(0..nbr_elements, i,
                    "self." mutator_i(i) ".complexity(value." i ", &" ti(i) ")"
                , separator: "+") ";"
            "Self::Cache {"
                join_ts!(0..nbr_elements, i, ti(i) ",")
                "cplx
            }
        }
        fn initial_step_from_value<'a>(&'a self, value: " tuple_ref ") -> Self::MutationStep {"
            join_ts!(0..nbr_elements, i,
                "let" ti(i) "= self." mutator_i(i) ".initial_step_from_value(value." i ");"
            )
            "let step = 0;"
            "Self::MutationStep {"
                join_ts!(0..nbr_elements, i, ti(i) ",")
                "inner: vec![" join_ts!(0..nbr_elements, i, "InnerMutationStep::" Ti(i), separator: ",") "] ,
                step ,
            }
        }
        fn ordered_arbitrary(
            &self,
            step: &mut Self::ArbitraryStep,
            max_cplx: f64,
        ) -> " cm.Option "<(T, Self::Cache)> {
            if max_cplx < <Self as" cm.TupleMutator "<T , " cm.TupleN_ident "<" tuple_type_params "> > >::min_complexity(self) { 
                return " cm.None " 
            }
            " // TODO: actually write something that is ordered_arbitrary sense here
            cm.Some "  (self.random_arbitrary(max_cplx))
        }
        fn random_arbitrary(&self, max_cplx: f64) -> (T, Self::Cache) {"
            join_ts!(0..nbr_elements, i,
                "let mut" ti_value(i) ":" cm.Option "<_> =" cm.None ";"
                "let mut" ti_cache(i) ":" cm.Option "<_> =" cm.None ";"
            )
            "let mut indices = ( 0 .." nbr_elements ").collect::<" cm.Vec "<_>>();"
            cm.fastrand "::shuffle(&mut indices);"
            "let mut cplx = 0.0;
            for idx in indices.iter() {
                match idx {"
                join_ts!(0..nbr_elements, i,
                    i "=> {
                        let (value, cache) = self." mutator_i(i) ".random_arbitrary(max_cplx - cplx);
                        cplx += self." mutator_i(i) ".complexity(&value, &cache);
                        " ti_value(i) "= " cm.Some "(value);
                        " ti_cache(i) "= " cm.Some "(cache);
                    }"
                )
                    "_ => unreachable!() ,
                }
            }
            (
                T::new(
                    ("
                    join_ts!(0..nbr_elements, i,
                        ti_value(i) ".unwrap()"
                    , separator:",")
                    ")
                ),
                Self::Cache {"
                    join_ts!(0..nbr_elements, i,
                        ti(i) ":" ti_cache(i) ".unwrap() ,"
                    )
                    "cplx,
                },
            )
        }

        fn ordered_mutate<'a>(
            &'a self,
            value: " tuple_mut ",
            cache: &'a mut Self::Cache,
            step: &'a mut Self::MutationStep,
            max_cplx: f64,
        ) -> " cm.Option "<Self::UnmutateToken> {
            if max_cplx < <Self as" cm.TupleMutator "<T , " cm.TupleN_ident "<" tuple_type_params "> > >::min_complexity(self) { return " cm.None " }
            if step.inner.is_empty() {
                return " cm.None ";
            }
            let orig_step = step.step;
            step.step += 1;
            let current_cplx = " SelfAsTupleMutator "::complexity(self, " TupleNAsRefTypes "::get_ref_from_mut(&value), cache); 
            let inner_step_to_remove: usize;

            match step.inner[orig_step % step.inner.len()] {"
            join_ts!(0..nbr_elements, i,
                "InnerMutationStep::" Ti(i) "=> {
                    let current_field_cplx = self." mutator_i(i) ".complexity(value." i ", &cache." ti(i) ");
                    let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                    if let " cm.Some "(token) =
                        self." mutator_i(i) "
                            .ordered_mutate(value." i ", &mut cache." ti(i) ", &mut step." ti(i) ", max_field_cplx)
                    {
                        let new_field_complexity = self." mutator_i(i) ".complexity(value." i ", &cache." ti(i) ");
                        cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                        return " cm.Some "(Self::UnmutateToken {
                            " ti(i) ": " cm.Some "(token),
                            cplx: current_cplx,
                            ..Self::UnmutateToken::default()
                        });
                    } else {
                        inner_step_to_remove = orig_step % step.inner.len();
                    }
                }"
            )"
            }
            step.inner.remove(inner_step_to_remove);
            " SelfAsTupleMutator "::ordered_mutate(self, value, cache, step, max_cplx)
        }
        "
        // TODO!
        "
        fn random_mutate<'a>(&'a self, value: " tuple_mut ", cache: &'a mut Self::Cache, max_cplx: f64, ) -> Self::UnmutateToken {
            let current_cplx = " SelfAsTupleMutator "::complexity(self, " TupleNAsRefTypes "::get_ref_from_mut(&value), cache);
            match self.rng.usize(.." nbr_elements ") {"
                join_ts!(0..nbr_elements, i,
                    i "=> {
                        let current_field_cplx = self." mutator_i(i) ".complexity(value." i ", &cache." ti(i) ");
                        let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                        let token = self." mutator_i(i) "
                            .random_mutate(value." i ", &mut cache." ti(i) ", max_field_cplx) ;
                    
                        let new_field_complexity = self." mutator_i(i) ".complexity(value." i ", &cache." ti(i) ");
                        cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                        return Self::UnmutateToken {
                            " ti(i) ": " cm.Some "(token),
                            cplx: current_cplx,
                            ..Self::UnmutateToken::default()
                        };
                    }"
                )
                "_ => unreachable!() ,
            }
        }
        fn unmutate<'a>(&'a self, value: " tuple_mut ", cache: &'a mut Self::Cache, t: Self::UnmutateToken) {
            cache.cplx = t.cplx;"
            join_ts!(0..nbr_elements, i,
                "if let" cm.Some "(subtoken) = t." ti(i) "{
                    self. " mutator_i(i) ".unmutate(value." i ", &mut cache. " ti(i) " , subtoken);
                }"
            )
        "}
    }
    "
    )
}

#[allow(non_snake_case)]
fn impl_default_mutator_for_tuple(tb: &mut TokenBuilder, nbr_elements: usize) {
    let cm = Common::new(nbr_elements);

    let Ti = cm.Ti.as_ref();
    let Mi = cm.Mi.as_ref();

    let tuple_type_params = join_ts!(0..nbr_elements, i, Ti(i), separator: ",");
    let mutator_type_params = join_ts!(0..nbr_elements, i, Mi(i), separator: ",");
    let type_params = ts!(tuple_type_params "," mutator_type_params);

    let TupleN = ts!(ident!("Tuple" nbr_elements) "<" tuple_type_params ">");
    let TupleMutatorWrapper = ts!(
        cm.TupleMutatorWrapper "<
            (" tuple_type_params "),"
            cm.TupleNMutator_ident "<"
                tuple_type_params ", "
                join_ts!(0..nbr_elements, i,
                    "<" Ti(i) "as" cm.DefaultMutator "> :: Mutator"
                , separator: ",")
            ">,"
            TupleN
        ">"
    );

    extend_ts!(tb,
    "
    impl<" type_params ">" cm.Default "for" cm.TupleNMutator_ident "<" type_params ">
        where"
        join_ts!(0..nbr_elements, i,
            Ti(i) ":" cm.Clone ","
            Mi(i) ":" cm.fuzzcheck_traits_Mutator "<" Ti(i) ">,"
        )
        join_ts!(0..nbr_elements, i, Mi(i) ":" cm.Default, separator: ",")
    "{
        fn default() -> Self {
            Self::new("
                join_ts!(0..nbr_elements, i,
                    "<" Mi(i) "as" cm.Default "> :: default()"
                , separator: ",")
            ")
        }
    } 

    impl<" tuple_type_params ">" cm.DefaultMutator "for (" tuple_type_params ")
        where" join_ts!(0..nbr_elements, i, Ti(i) ":" cm.DefaultMutator "+ 'static", separator: ",")
    "{
        type Mutator = " TupleMutatorWrapper ";
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

#[cfg(test)]
mod test {
    use decent_synquote_alternative::TokenBuilderExtend;
    use decent_synquote_alternative::{parser::TokenParser, token_builder};
    use proc_macro2::TokenStream;
    use token_builder::TokenBuilder;

    use crate::MakeMutatorSettings;

    use super::{
        declare_tuple_mutator, declare_tuple_mutator_helper_types, /*impl_default_mutator_for_struct_with_1_field,*/
        impl_default_mutator_for_struct_with_more_than_1_field, impl_default_mutator_for_tuple, impl_mutator_trait,
        impl_tuple_structure_trait, /*impl_wrapped_tuple_1_structure,*/ make_tuple_type_structure,
    };

    // #[test]
    // fn test_make_mutator_with_forced_field_mutator() {
    //     let mut tb = TokenBuilder::new();
    //     let code = "
    //     struct S {
    //         x: u8,
    //         #[field_mutator(OptionMutator<Box<S>, BoxMutator<S, Weak<SMutator<M0>>>> = { OptionMutator::new(BoxMutator::new(self_.clone())) })]
    //         y: Option<Box<S>>,
    //     }"
    //     .parse::<TokenStream>()
    //     .unwrap();
    //     let mut parser = TokenParser::new(code);
    //     let struc = parser.eat_struct().unwrap();
    //     let settings = MakeMutatorSettings {
    //         recursive: true,  
    //         ..<_>::default()
    //     };
    //     impl_default_mutator_for_struct_with_more_than_1_field(&mut tb, &struc, &settings);
    //     let generated = tb.end().to_string();
    //
    //     // let expected = "
    //     // impl<T> DefaultMutator for XY<T> where 
    //     //     VecMutator<XY<T>, Weak<Self>> : Default // if there's no equal
    //     // {
    //     //     fn default_mutator() -> Self::Mutator {
    //     //         Self::Mutator(T::default_mutator(), { VecMutator::new(10) })
    //     //         Self::Mutator(T::default_mutator(), <VecMutator<XY<T>, Weak<Self>>>::default() })
    //     //         Rc::new_cyclic(|self_| Self::Mutator(T::default_mutator(), { VecMutator::new(self_) }) )
    //     //     }
    //     // }";
    //
    //     assert!(false, "\n\n{}\n\n", generated);
    // }

    #[test]
    fn test_impl_default_mutator_two_fields() {
        let mut tb = TokenBuilder::new();
        let code = "
        pub struct S<T: Into<u8>> where T: Default {
            x: u8,
            y: Vec<T> ,
        }
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let struc = parser.eat_struct().unwrap();
        impl_default_mutator_for_struct_with_more_than_1_field(&mut tb, &struc, &<_>::default());
        let generated = tb.end().to_string();

        let expected = "
        pub struct SMutator < T : Into < u8 >, M0 , M1 > where 
            T : Default , 
            T : :: std :: clone :: Clone + 'static , 
            M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
            M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            pub mutator : fuzzcheck_mutators :: TupleMutatorWrapper < 
                S < T > , 
                fuzzcheck_mutators :: Tuple2Mutator < 
                    u8 , Vec < T > , M0 , M1 
                >, 
                fuzzcheck_mutators :: Tuple2 < u8 , Vec < T > > 
            > 
        } 
        pub struct SMutatorCache < T : Into < u8 >, M0 , M1 > where 
            T : Default , 
            T : :: std :: clone :: Clone + 'static , 
            M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
            M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            inner : < 
                fuzzcheck_mutators :: TupleMutatorWrapper < 
                    S < T > , 
                    fuzzcheck_mutators :: Tuple2Mutator < u8 , Vec < T > , M0 , M1 >, 
                    fuzzcheck_mutators :: Tuple2 < u8 , Vec < T > > 
            > as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: Cache , 
        } 
        impl < T : Into < u8 >, M0 , M1 > SMutatorCache < T , M0 , M1 > where 
            T : Default , 
            T : :: std :: clone :: Clone + 'static , 
            M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
            M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            fn new (inner : < fuzzcheck_mutators :: TupleMutatorWrapper < S < T > , fuzzcheck_mutators :: Tuple2Mutator < u8 , Vec < T > , M0 , M1 >, fuzzcheck_mutators :: Tuple2 < u8 , Vec < T > > > as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: Cache) -> Self 
            {
                 Self { inner : (inner) } 
            } 
        } 
        impl < T : Into < u8 >, M0 , M1 > :: std :: clone :: Clone for SMutatorCache < T , M0 , M1 > where T : Default , T : :: std :: clone :: Clone + 'static , M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            fn clone (& self) -> Self { Self { inner : self . inner . clone () } } 
        } 
        pub struct SMutatorMutationStep < T : Into < u8 >, M0 , M1 > where T : Default , T : :: std :: clone :: Clone + 'static , M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            inner : < fuzzcheck_mutators :: TupleMutatorWrapper < 
                S < T > , 
                fuzzcheck_mutators :: Tuple2Mutator < u8 , Vec < T > , M0 , M1 >, 
                fuzzcheck_mutators :: Tuple2 < u8 , Vec < T > > 
            > as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: MutationStep , 
        } 
        impl < T : Into < u8 >, M0 , M1 > SMutatorMutationStep < T , M0 , M1 > where T : Default , T : :: std :: clone :: Clone + 'static , M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            fn new (inner : < fuzzcheck_mutators :: TupleMutatorWrapper < S < T > , fuzzcheck_mutators :: Tuple2Mutator < u8 , Vec < T > , M0 , M1 >, fuzzcheck_mutators :: Tuple2 < u8 , Vec < T > > > as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: MutationStep) -> Self { 
                Self { inner : (inner) } 
            } 
        } 
        impl < T : Into < u8 >, M0 , M1 > :: std :: clone :: Clone for SMutatorMutationStep < T , M0 , M1 > where T : Default , T : :: std :: clone :: Clone + 'static , M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            fn clone (& self) -> Self { 
                Self { inner : self . inner . clone () } 
            } 
        } 
        pub struct SMutatorArbitraryStep < T : Into < u8 >, M0 , M1 > where T : Default , T : :: std :: clone :: Clone + 'static , M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            inner : < fuzzcheck_mutators :: TupleMutatorWrapper < S < T > , fuzzcheck_mutators :: Tuple2Mutator < u8 , Vec < T > , M0 , M1 >, fuzzcheck_mutators :: Tuple2 < u8 , Vec < T > > > as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: ArbitraryStep , 
        } 
        impl < T : Into < u8 >, M0 , M1 > SMutatorArbitraryStep < T , M0 , M1 > where T : Default , T : :: std :: clone :: Clone + 'static , M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            fn new (inner : < fuzzcheck_mutators :: TupleMutatorWrapper < S < T > , fuzzcheck_mutators :: Tuple2Mutator < u8 , Vec < T > , M0 , M1 >, fuzzcheck_mutators :: Tuple2 < u8 , Vec < T > > > as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: ArbitraryStep) -> Self 
            { 
                Self { inner : (inner) } 
            } 
        } 
        impl < T : Into < u8 >, M0 , M1 > :: std :: clone :: Clone for SMutatorArbitraryStep < T , M0 , M1 > where T : Default , T : :: std :: clone :: Clone + 'static , M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            fn clone (& self) -> Self { Self { inner : self . inner . clone () } } 
        } 
        impl < T : Into < u8 >, M0 , M1 > :: std :: default :: Default for SMutatorArbitraryStep < T , M0 , M1 > where T : Default , T : :: std :: clone :: Clone + 'static , M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            fn default () -> Self { Self { inner : < _ >:: default () } } 
        } 
        pub struct SMutatorUnmutateToken < T : Into < u8 >, M0 , M1 > where T : Default , T : :: std :: clone :: Clone + 'static , M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            inner : < fuzzcheck_mutators :: TupleMutatorWrapper < S < T > , fuzzcheck_mutators :: Tuple2Mutator < u8 , Vec < T > , M0 , M1 >, fuzzcheck_mutators :: Tuple2 < u8 , Vec < T > > > as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: UnmutateToken , 
        } 
        impl < T : Into < u8 >, M0 , M1 > SMutatorUnmutateToken < T , M0 , M1 > where T : Default , T : :: std :: clone :: Clone + 'static , M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            fn new (inner : < fuzzcheck_mutators :: TupleMutatorWrapper < S < T > , fuzzcheck_mutators :: Tuple2Mutator < u8 , Vec < T > , M0 , M1 >, fuzzcheck_mutators :: Tuple2 < u8 , Vec < T > > > as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: UnmutateToken) -> Self 
            { 
                Self { inner : (inner) } 
            } 
        } 
        impl < T : Into < u8 >, M0 , M1 > SMutator < T , M0 , M1 > where T : Default , T : :: std :: clone :: Clone + 'static , M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            pub fn new (mutator_x : M0 , mutator_y : M1) -> Self { 
                Self { 
                    mutator : fuzzcheck_mutators :: TupleMutatorWrapper :: new (fuzzcheck_mutators :: Tuple2Mutator :: new (mutator_x , mutator_y)) 
                } 
            } 
        } 
        impl < T : Into < u8 >, M0 , M1 > :: std :: default :: Default for SMutator < T , M0 , M1 > where 
            T : Default , 
            T : :: std :: clone :: Clone + 'static , 
            M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
            M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > , 
            fuzzcheck_mutators :: TupleMutatorWrapper < S < T > , fuzzcheck_mutators :: Tuple2Mutator < u8 , Vec < T > , M0 , M1 >, fuzzcheck_mutators :: Tuple2 < u8 , Vec < T > > > : :: std :: default :: Default 
        { 
            fn default () -> Self { Self { mutator : < _ >:: default () } } 
        } 
        impl < T : Into < u8 >, M0 , M1 > fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > for SMutator < T , M0 , M1 > where 
            T : Default , 
            T : :: std :: clone :: Clone + 'static , 
            M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
            M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
        { 
            type Cache = SMutatorCache < T , M0 , M1 > ; 
            type ArbitraryStep = SMutatorArbitraryStep < T , M0 , M1 > ; 
            type MutationStep = SMutatorMutationStep < T , M0 , M1 > ; 
            type UnmutateToken = SMutatorUnmutateToken < T , M0 , M1 > ; 
            fn cache_from_value (& self , value : & S < T >) -> Self :: Cache { 
                Self :: Cache :: new (self . mutator . cache_from_value (value)) 
            } 
            fn initial_step_from_value (& self , value : & S < T >) -> Self :: MutationStep { 
                Self :: MutationStep :: new (self . mutator . initial_step_from_value (value)) 
            } 
            fn max_complexity (& self) -> f64 { 
                self . mutator . max_complexity () 
            } 
            fn min_complexity (& self) -> f64 { 
                self . mutator . min_complexity () 
            } 
            fn complexity (& self , value : & S < T > , cache : & Self :: Cache) -> f64 { 
                self . mutator . complexity (value , & cache . inner) 
            } 
            fn ordered_arbitrary (& self , step : & mut Self :: ArbitraryStep , max_cplx : f64) -> Option < (S < T > , Self :: Cache) > { 
                if let :: std :: option :: Option :: Some ((value , cache)) = self . mutator . ordered_arbitrary (& mut step . inner , max_cplx) { 
                    :: std :: option :: Option :: Some ((value , Self :: Cache :: new (cache))) 
                } else { 
                    :: std :: option :: Option :: None 
                } 
            } 
            fn random_arbitrary (& self , max_cplx : f64) -> (S < T > , Self :: Cache) {
                let (value , cache) = self . mutator . random_arbitrary (max_cplx) ; 
                (value , Self :: Cache :: new (cache)) 
            } 
            fn ordered_mutate (& self , value : & mut S < T > , cache : & mut Self :: Cache , step : & mut Self :: MutationStep , max_cplx : f64 ,) -> Option < Self :: UnmutateToken > { 
                if let :: std :: option :: Option :: Some (t) = self . mutator . ordered_mutate (value , & mut cache . inner , & mut step . inner , max_cplx) { 
                    :: std :: option :: Option :: Some (Self :: UnmutateToken :: new (t)) 
                } else { 
                    :: std :: option :: Option :: None 
                } 
            } 
            fn random_mutate (& self , value : & mut S < T > , cache : & mut Self :: Cache , max_cplx : f64) -> Self :: UnmutateToken { 
                Self :: UnmutateToken :: new (self . mutator . random_mutate (value , & mut cache . inner , max_cplx)) 
            } 
            fn unmutate (& self , value : & mut S < T > , cache : & mut Self :: Cache , t : Self :: UnmutateToken) { 
                self . mutator . unmutate (value , & mut cache . inner , t . inner) } 
            } 
            impl < T : Into < u8 >> fuzzcheck_mutators :: DefaultMutator for S < T > where 
                T : Default , 
                T : fuzzcheck_mutators :: DefaultMutator + 'static 
            { 
                type Mutator = SMutator < T , < u8 as fuzzcheck_mutators :: DefaultMutator >:: Mutator , < Vec < T > as fuzzcheck_mutators :: DefaultMutator >:: Mutator > ; 
                fn default_mutator () -> Self :: Mutator { 
                    SMutator :: new (< u8 >:: default_mutator () , < Vec < T > >:: default_mutator () ,) 
                } 
            } 
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_default_mutator_for_tuple() {
        let mut tb = TokenBuilder::new();
        impl_default_mutator_for_tuple(&mut tb, 2);
        let generated = tb.end().to_string();

        let expected = "
impl<T0, T1, M0, M1> ::std::default::Default for Tuple2Mutator<T0, T1, M0, M1>
where
    T0: ::std::clone::Clone,
    M0: ::fuzzcheck_traits::Mutator<T0>,
    T1: ::std::clone::Clone,
    M1: ::fuzzcheck_traits::Mutator<T1>,
    M0: ::std::default::Default,
    M1: ::std::default::Default
{
    fn default() -> Self {
        Self::new(
            <M0 as ::std::default::Default> ::default(),
            <M1 as ::std::default::Default> ::default()
        )
    }
}
impl<T0, T1> fuzzcheck_mutators::DefaultMutator for (T0, T1)
where
    T0: fuzzcheck_mutators::DefaultMutator + 'static,
    T1: fuzzcheck_mutators::DefaultMutator + 'static
{
    type Mutator = fuzzcheck_mutators::TupleMutatorWrapper<
        (T0, T1),
        Tuple2Mutator<T0, T1, <T0 as fuzzcheck_mutators::DefaultMutator> ::Mutator, <T1 as fuzzcheck_mutators::DefaultMutator> ::Mutator>,
        Tuple2<T0, T1>
    > ;

    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(Tuple2Mutator::new(
            <T0 as fuzzcheck_mutators::DefaultMutator> ::default_mutator(),
            <T1 as fuzzcheck_mutators::DefaultMutator> ::default_mutator()
        ))
    }
}
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}", generated, expected);
    }


    #[test]
    fn test_impl_tuple_structure_trait_one_field_generics() {
        let code = "
        pub struct Y {
            x: bool,
        }
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let struc = parser.eat_struct().unwrap();

        let mut tb = TokenBuilder::new();

        let mut settings = MakeMutatorSettings::default();
        settings.fuzzcheck_mutators_crate = ts!("fuzzcheck_mutators");
        impl_tuple_structure_trait(&mut tb, &struc);

        let generated = tb.end().to_string();

        let expected = "
        impl fuzzcheck_mutators :: TupleStructure < fuzzcheck_mutators :: Wrapped < bool > > for Y where 
        { 
            fn get_ref <'a > (&'a self) -> (&'a bool) { 
                (& self . x) 
            } 
            fn get_mut <'a > (&'a mut self) -> (&'a mut bool) { 
                (& mut self . x) 
            } 
            fn new (t : (bool)) -> Self { 
                Self { x : t } 
            } 
        }"
        .parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}", generated, expected);
    }

    #[test]
    fn test_impl_tuple_structure_trait_generics() {
        let code = "
        pub struct A <T, U: Clone = u8> where T: Default {
            x: u8,
            y: Vec<(T, U)> ,
        }
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let struc = parser.eat_struct().unwrap();

        let mut tb = TokenBuilder::new();

        let mut settings = MakeMutatorSettings::default();
        settings.fuzzcheck_mutators_crate = ts!("crate");
        impl_tuple_structure_trait(&mut tb, &struc);

        let generated = tb.end().to_string();

        let expected = "  
impl<T, U: Clone> fuzzcheck_mutators::TupleStructure<fuzzcheck_mutators::Tuple2< u8, Vec<(T, U)> > > 
    for A <T, U> 
    where 
        T: Default, 
        T: 'static, 
        U: 'static 
{
    fn get_ref<'a>(&'a self) -> (&'a u8, &'a Vec<(T, U)> ) {
        (&self.x, &self.y)
    }
    fn get_mut<'a>(&'a mut self) -> (&'a mut u8, &'a mut Vec<(T, U)> ) {
        (&mut self.x, &mut self.y)
    }

    fn new(t: (u8, Vec<(T, U)> )) -> Self {
        Self {
            x: t.0,
            y: t.1
        }
    }
}
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}", generated, expected);
    }

    #[test]
    fn test_impl_mutator_trait() {
        let mut tb = TokenBuilder::new();
        impl_mutator_trait(&mut tb, 2);
        let generated = tb.end().to_string();

        let expected = "
impl<T, T0, T1, M0, M1> fuzzcheck_mutators::TupleMutator<T, Tuple2<T0, T1> > for Tuple2Mutator<T0, T1, M0, M1>
    where
    T: ::std::clone::Clone,
    T0: ::std::clone::Clone + 'static,
    M0: ::fuzzcheck_traits::Mutator<T0>,
    T1: ::std::clone::Clone + 'static,
    M1: ::fuzzcheck_traits::Mutator<T1>,
    T: fuzzcheck_mutators::TupleStructure<Tuple2<T0, T1> >,
{
    type Cache = Cache< <M0 as ::fuzzcheck_traits::Mutator<T0> >::Cache, <M1 as ::fuzzcheck_traits::Mutator<T1> >::Cache>;
    type MutationStep = MutationStep<
        <M0 as ::fuzzcheck_traits::Mutator<T0> >::MutationStep,
        <M1 as ::fuzzcheck_traits::Mutator<T1> >::MutationStep
    >;
    type ArbitraryStep = ArbitraryStep<
        <M0 as ::fuzzcheck_traits::Mutator<T0> >::ArbitraryStep,
        <M1 as ::fuzzcheck_traits::Mutator<T1> >::ArbitraryStep
    >;
    type UnmutateToken = UnmutateToken<
        <M0 as ::fuzzcheck_traits::Mutator<T0> >::UnmutateToken,
        <M1 as ::fuzzcheck_traits::Mutator<T1> >::UnmutateToken
    >;

    fn max_complexity(&self) -> f64 {
        self.mutator_0.max_complexity() + self.mutator_1.max_complexity()
    }
    fn min_complexity(&self) -> f64 {
        self.mutator_0.min_complexity() + self.mutator_1.min_complexity()
    }
    fn complexity<'a>(&'a self, _value: (&'a T0, &'a T1), cache: &'a Self::Cache) -> f64 {
        cache.cplx
    }
    fn cache_from_value<'a>(&'a self, value: (&'a T0, &'a T1)) -> Self::Cache {
        let t0 = self.mutator_0.cache_from_value(value.0);
        let t1 = self.mutator_1.cache_from_value(value.1);
        let cplx = self.mutator_0.complexity(value.0, &t0) + self.mutator_1.complexity(value.1, &t1);
        Self::Cache { t0, t1, cplx }
    }
    fn initial_step_from_value<'a>(&'a self, value: (&'a T0, &'a T1)) -> Self::MutationStep {
        let t0 = self.mutator_0.initial_step_from_value(value.0);
        let t1 = self.mutator_1.initial_step_from_value(value.1);
        let step = 0;
        Self::MutationStep {
            t0,
            t1,
            inner: vec![InnerMutationStep::T0, InnerMutationStep::T1],
            step,
        }
    }
    fn ordered_arbitrary(
        &self,
        step: &mut Self::ArbitraryStep,
        max_cplx: f64,
    ) -> ::std::option::Option<(T, Self::Cache)> {
        if max_cplx < <Self as fuzzcheck_mutators::TupleMutator<T, Tuple2<T0, T1> > >::min_complexity(self) { return ::std::option::Option::None }
        ::std::option::Option::Some(self.random_arbitrary(max_cplx))
    }
    fn random_arbitrary(&self, max_cplx: f64) -> (T, Self::Cache) {
        let mut t0_value: ::std::option::Option<_> = ::std::option::Option::None;
        let mut t0_cache: ::std::option::Option<_> = ::std::option::Option::None;
        let mut t1_value: ::std::option::Option<_> = ::std::option::Option::None;
        let mut t1_cache: ::std::option::Option<_> = ::std::option::Option::None;
        let mut indices = (0..2).collect::< ::std::vec::Vec<_>>();
        fuzzcheck_mutators::fastrand::shuffle(&mut indices);

        let mut cplx = 0.0;
        for idx in indices.iter() {
            match idx {
                0 => {
                    let (value, cache) = self.mutator_0.random_arbitrary(max_cplx - cplx);
                    cplx += self.mutator_0.complexity(&value, &cache);
                    t0_value = ::std::option::Option::Some(value);
                    t0_cache = ::std::option::Option::Some(cache);
                }
                1 => {
                    let (value, cache) = self.mutator_1.random_arbitrary(max_cplx - cplx);
                    cplx += self.mutator_1.complexity(&value, &cache);
                    t1_value = ::std::option::Option::Some(value);
                    t1_cache = ::std::option::Option::Some(cache);
                }
                _ => unreachable!(),
            }
        }
        (
            T::new((t0_value.unwrap(), t1_value.unwrap())),
            Self::Cache {
                t0: t0_cache.unwrap(),
                t1: t1_cache.unwrap(),
                cplx,
            },
        )
    }

    fn ordered_mutate<'a>(
        &'a self,
        value: (&'a mut T0, &'a mut T1),
        cache: &'a mut Self::Cache,
        step: &'a mut Self::MutationStep,
        max_cplx: f64,
    ) -> ::std::option::Option<Self::UnmutateToken> {
        if max_cplx < <Self as fuzzcheck_mutators::TupleMutator<T, Tuple2<T0, T1> > >::min_complexity(self) { return ::std::option::Option::None }        
        if step.inner.is_empty() {
            return ::std::option::Option::None;
        }
        let orig_step = step.step;
        step.step += 1;
        let current_cplx =
            <Self as fuzzcheck_mutators::TupleMutator<T,Tuple2<T0, T1> >> ::complexity(self, <Tuple2<T0, T1> as fuzzcheck_mutators::RefTypes> ::get_ref_from_mut(&value), cache);
        let inner_step_to_remove: usize;
        // TODO: add complexity to steps array to spend more resources on complex elements
        match step.inner[orig_step % step.inner.len()] {
            InnerMutationStep::T0 => {
                let current_field_cplx = self.mutator_0.complexity(value.0, &cache.t0);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                if let ::std::option::Option::Some(token) =
                    self.mutator_0
                        .ordered_mutate(value.0, &mut cache.t0, &mut step.t0, max_field_cplx)
                {
                    let new_field_complexity = self.mutator_0.complexity(value.0, &cache.t0);
                    cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                    return ::std::option::Option::Some(Self::UnmutateToken {
                        t0: ::std::option::Option::Some(token),
                        cplx: current_cplx,
                        ..Self::UnmutateToken::default()
                    });
                } else {
                    inner_step_to_remove = orig_step % step.inner.len();
                }
            }
            InnerMutationStep::T1 => {
                let current_field_cplx = self.mutator_1.complexity(value.1, &cache.t1);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                if let ::std::option::Option::Some(token) =
                    self.mutator_1
                        .ordered_mutate(value.1, &mut cache.t1, &mut step.t1, max_field_cplx)
                {
                    let new_field_complexity = self.mutator_1.complexity(value.1, &cache.t1);
                    cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                    return ::std::option::Option::Some(Self::UnmutateToken {
                        t1: ::std::option::Option::Some(token),
                        cplx: current_cplx,
                        ..Self::UnmutateToken::default()
                    });
                } else {
                    inner_step_to_remove = orig_step % step.inner.len();
                }
            }
        }
        step.inner.remove(inner_step_to_remove);
        <Self as fuzzcheck_mutators::TupleMutator<T,Tuple2<T0, T1> >> ::ordered_mutate(self, value, cache, step, max_cplx)
    }
    fn random_mutate<'a>(
        &'a self,
        value: (&'a mut T0, &'a mut T1),
        cache: &'a mut Self::Cache,
        max_cplx: f64,
    ) -> Self::UnmutateToken {
        let current_cplx =
            <Self as fuzzcheck_mutators::TupleMutator<T,Tuple2<T0, T1> >> ::complexity(self, <Tuple2<T0, T1> as fuzzcheck_mutators::RefTypes> ::get_ref_from_mut(&value), cache);
        match self.rng.usize(..2) {
            0 => {
                let current_field_cplx = self.mutator_0.complexity(value.0, &cache.t0);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                let token = self.mutator_0.random_mutate(value.0, &mut cache.t0, max_field_cplx);
                let new_field_complexity = self.mutator_0.complexity(value.0, &cache.t0);
                cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                return Self::UnmutateToken {
                    t0: ::std::option::Option::Some(token),
                    cplx: current_cplx,
                    ..Self::UnmutateToken::default()
                };
            }
            1 => {
                let current_field_cplx = self.mutator_1.complexity(value.1, &cache.t1);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                let token = self.mutator_1.random_mutate(value.1, &mut cache.t1, max_field_cplx);
                let new_field_complexity = self.mutator_1.complexity(value.1, &cache.t1);
                cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                return Self::UnmutateToken {
                    t1: ::std::option::Option::Some(token),
                    cplx: current_cplx,
                    ..Self::UnmutateToken::default()
                };
            }
            _ => unreachable!(),
        }
    }
    fn unmutate<'a>(&'a self, value: (&'a mut T0, &'a mut T1), cache: &'a mut Self::Cache, t: Self::UnmutateToken) {
        cache.cplx = t.cplx;
        if let ::std::option::Option::Some(subtoken) = t.t0 {
            self.mutator_0.unmutate(value.0, &mut cache.t0, subtoken);
        }
        if let ::std::option::Option::Some(subtoken) = t.t1 {
            self.mutator_1.unmutate(value.1, &mut cache.t1, subtoken);
        }
    }
}
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}", generated, expected);
    }

    #[test]
    fn test_declare_tuple_mutator_helper_types() {
        let mut tb = TokenBuilder::new();
        declare_tuple_mutator_helper_types(&mut tb, 2);
        let generated = tb.end().to_string();

        let expected = r#"
#[derive(::std::clone::Clone)]
pub struct Cache<T0, T1> {
    t0: T0,
    t1: T1,
    cplx: f64
}
#[derive(::std::clone::Clone)]
pub enum InnerMutationStep {
    T0,
    T1
}
#[derive(::std::clone::Clone)]
pub struct MutationStep<T0, T1> {
    t0: T0,
    t1: T1,
    step: usize,
    inner: ::std::vec::Vec<InnerMutationStep>
}

#[derive(::std::default::Default, ::std::clone::Clone)]
pub struct ArbitraryStep<T0, T1> {
    t0: T0,
    t1: T1
}

pub struct UnmutateToken<T0, T1> {
    t0: ::std::option::Option<T0> ,
    t1: ::std::option::Option<T1> ,
    cplx: f64
}
impl<T0, T1> ::std::default::Default for UnmutateToken<T0, T1> {
    fn default() -> Self {
        Self {
            t0: ::std::option::Option ::None ,
            t1: ::std::option::Option ::None ,
            cplx: <_>::default()
        }
    }
}
        "#
        .parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}", generated, expected);
    }

    #[test]
    fn test_declare_tuple_mutator() {
        let mut tb = TokenBuilder::new();
        declare_tuple_mutator(&mut tb, 2);
        let generated = tb.end().to_string();

        let expected = r#"  
pub struct Tuple2Mutator<T0, T1, M0, M1>
where
    T0: ::std::clone::Clone,
    M0: ::fuzzcheck_traits::Mutator<T0> ,
    T1: ::std::clone::Clone,
    M1: ::fuzzcheck_traits::Mutator<T1>
{
    pub mutator_0: M0,
    pub mutator_1: M1,
    rng: fuzzcheck_mutators::fastrand::Rng,
    _phantom: ::std::marker::PhantomData<(T0, T1)>
}
impl<T0, T1, M0, M1> Tuple2Mutator<T0, T1, M0, M1>
where
    T0: ::std::clone::Clone,
    M0: ::fuzzcheck_traits::Mutator<T0> ,
    T1: ::std::clone::Clone,
    M1: ::fuzzcheck_traits::Mutator<T1>
{
    pub fn new(mutator_0: M0, mutator_1: M1) -> Self {
        Self {
            mutator_0,
            mutator_1,
            rng: <_>::default() ,
            _phantom: ::std::marker::PhantomData
        }
    }

    pub fn replacing_mutator_0<M>(self, mutator: M) -> Tuple2Mutator<T0, T1, M, M1>
    where
        M: ::fuzzcheck_traits::Mutator<T0>
    {
        Tuple2Mutator {
            mutator_0: mutator,
            mutator_1: self.mutator_1,
            rng: self.rng,
            _phantom: self._phantom
        }
    }
    pub fn replacing_mutator_1<M>(self, mutator: M) -> Tuple2Mutator<T0, T1, M0, M>
    where
        M: ::fuzzcheck_traits::Mutator<T1>
    {
        Tuple2Mutator {
            mutator_0: self.mutator_0,
            mutator_1: mutator,
            rng: self.rng,
            _phantom: self._phantom
        }
    }
}
        "#
        .parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}", generated, expected);
    }

    #[test]
    fn test_make_tuple_type_structure() {
        let mut tb = TokenBuilder::new();
        make_tuple_type_structure(&mut tb, 2);
        let generated = tb.end().to_string();
        let expected = r#"
        pub struct Tuple2<T0: 'static, T1: 'static> {
            _phantom: ::std::marker::PhantomData<(T0, T1)> ,
        }
        impl<T0: 'static, T1: 'static> fuzzcheck_mutators::RefTypes for Tuple2<T0, T1> {
            type Owned = (T0, T1);
            type Ref<'a> = (&'a T0, &'a T1);
            type Mut<'a> = (&'a mut T0, &'a mut T1);
            fn get_ref_from_mut<'a>(v: &'a Self::Mut<'a>) -> Self::Ref<'a> {
                (v.0, v.1)
            }
        }
        impl<T0: 'static, T1: 'static> fuzzcheck_mutators::TupleStructure<Tuple2<T0, T1> > for (T0, T1) {
            fn get_ref<'a>(&'a self) -> (&'a T0, &'a T1) {
                (&self.0, &self.1)
            }
            fn get_mut<'a>(&'a mut self) -> (&'a mut T0, &'a mut T1) {
                (&mut self.0, &mut self.1)
            }
            fn new(t: (T0, T1)) -> Self {
                t
            }
        }
        "#
        .parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}", generated, expected);
    }
}
