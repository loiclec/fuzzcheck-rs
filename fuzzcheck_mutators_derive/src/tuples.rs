use decent_synquote_alternative as synquote;
use proc_macro2::{Ident, Span, TokenStream};

use synquote::parser::*;
use synquote::token_builder::*;

use crate::structs_and_enums::{FieldMutator, FieldMutatorKind};
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

    let TupleKind = cm.TupleN_path.clone();

    let generics_no_eq = struc.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = struc.generics.removing_bounds_and_eq_type();

    let tuple_owned = ts!("(" join_ts!(&struc.struct_fields, field, field.ty , separator: ",") ")");
    let tuple_ref = ts!("(" join_ts!(&struc.struct_fields, field, "&'a" field.ty , separator: ",") ")");
    let tuple_mut = ts!("(" join_ts!(&struc.struct_fields, field, "&'a mut" field.ty , separator: ",") ")");

    let mut where_clause = struc.where_clause.clone().unwrap_or_default();
    where_clause.add_clause_items(join_ts!(&struc.generics.type_params, tp,
        tp.type_ident ": 'static,"
    ));

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

pub(crate) fn impl_default_mutator_for_struct_with_0_field(tb: &mut TokenBuilder, struc: &Struct) {
    assert!(struc.struct_fields.is_empty());
    let cm = Common::new(0);
    let generics_no_eq = struc.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = struc.generics.removing_bounds_and_eq_type();

    // add T: DefaultMutator for each generic type parameter to the existing where clause
    let mut where_clause = struc.where_clause.clone().unwrap_or_default();
    where_clause.add_clause_items(join_ts!(&struc.generics.type_params, ty_param,
        ty_param ":" cm.DefaultMutator ","
    ));

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

#[allow(non_snake_case)]
pub(crate) fn impl_default_mutator_for_struct(tb: &mut TokenBuilder, struc: &Struct, settings: &MakeMutatorSettings) {
    let nbr_elements = struc.struct_fields.len();

    let cm = Common::new(nbr_elements);
    let TupleNMutator = cm.TupleNMutator.as_ref()(nbr_elements);

    let field_types = join_ts!(&struc.struct_fields, field, field.ty, separator: ",");

    let field_mutators = vec![struc
        .struct_fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let mut mutator = None;
            for attribute in field.attributes.iter() {
                if let Some((m, init)) = super::read_field_default_mutator_attribute(attribute.clone()) {
                    mutator = Some((m, init));
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
            struc.ident struc.generics.removing_bounds_and_eq_type() ","
            TupleNMutator "<"
                field_types ", "
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
        visibility: &struc.visibility,
        type_ident: &struc.ident,
        type_generics: &struc.generics,
        type_where_clause: &struc.where_clause,
        field_mutators: &field_mutators,
        InnerMutator: &TupleMutatorWrapper,
        new_impl: &ts!(
            "pub fn new("
            join_ts!(struc.struct_fields.iter().zip(field_mutators.iter().flatten()), (field, mutator),
                ident!("mutator_" field.access()) ":" mutator.mutator_stream(&cm)
            , separator: ",")
            ") -> Self {
            Self {
                mutator : " cm.TupleMutatorWrapper "::new(" TupleNMutator "::new("
                    join_ts!(struc.struct_fields.iter(), field,
                        ident!("mutator_" field.access())
                    , separator: ",")
                    "))
            }
            }"
        ),
        default_impl: &ts!("
            fn default() -> Self {
                Self { mutator : <_>::default() }
            }
        "),
        settings,
    };

    extend_ts!(tb, make_mutator_type_and_impl(params));
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
fn declare_tuple_mutator_helper_types(tb: &mut TokenBuilder, nbr_elements: usize) {
    let cm = Common::new(nbr_elements);
    let Ti = cm.Ti.as_ref();
    let ti = cm.ti.as_ref();
    let tuple_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");

    extend_ts!(tb,
        "pub struct Cache <" tuple_type_params "> {"
            join_ts!(0..nbr_elements, i,
                ti(i) ":" ident!("T" i) ","
            )
            "cplx : f64,
            vose_alias : " cm.VoseAlias "
        }
        pub enum InnerMutationStep {"
            join_ts!(0..nbr_elements, i,
                Ti(i)
            , separator: ",")
        "}
        pub struct MutationStep < " tuple_type_params " > {"
            join_ts!(0..nbr_elements, i,
                ti(i) ":" Ti(i) ","
            )
            "inner : " cm.Vec " < InnerMutationStep > ,
            dead_ends : " cm.Vec "<bool> ,
            dead_end: bool,
        }
        pub struct ArbitraryStep < " tuple_type_params " > {"
            join_ts!(0..nbr_elements, i,
                ti(i) ":" Ti(i)
            , separator: ",")
        "}

        pub struct UnmutateToken < " tuple_type_params " > {"
            join_ts!(0..nbr_elements, i,
                ti(i) ":" cm.Option "<" Ti(i) "> ,"
            )
            "
        }
        impl < " tuple_type_params " > " cm.Default " for UnmutateToken < " tuple_type_params " > {
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
    let tuple_ref = ts!("(" join_ts!(0..nbr_elements, i, "&'a" Ti(i), separator: ",") ")");
    let tuple_mut = ts!("(" join_ts!(0..nbr_elements, i, "&'a mut" Ti(i), separator: ",") ")");

    let SelfAsTupleMutator = ts!("<Self as " cm.TupleMutator "<T, " cm.TupleN_ident "<" tuple_type_params "> >>");

    let TupleNAsRefTypes = ts!("<" cm.TupleN_ident "<" tuple_type_params "> as " cm.RefTypes ">");

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
        
        fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
            Self::ArbitraryStep {"
                join_ts!(0..nbr_elements, i,
                    ti(i) ": self." mutator_i(i) ".default_arbitrary_step()"
                , separator: ",")
            "}
        }
        
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
        
        fn validate_value<'a>(&'a self, value: " tuple_ref ") -> " cm.Option "<(Self::Cache, Self::MutationStep)> {"
            join_ts!(0..nbr_elements, i,
                "let (" ident!("c" i) ", " ident!("s" i) ") = self." mutator_i(i) ".validate_value(value." i ")?;"
            )
            join_ts!(0..nbr_elements, i,
                "let" ident!("cplx_" i) " = self." mutator_i(i) ".complexity(value." i ", &" ident!("c" i) ");"
            )

            "let sum_cplx = "
                join_ts!(0..nbr_elements, i,
                    ident!("cplx_" i)
                , separator: "+") ";

            let dead_ends = vec![" join_ts!(0 .. nbr_elements, _, "false,")  "];

            let step = Self::MutationStep {"
                join_ts!(0..nbr_elements, i, ti(i) ":" ident!("s" i) ",")
                "inner: vec![" join_ts!(0..nbr_elements, i, "InnerMutationStep::" Ti(i), separator: ",") "] ,
                dead_ends,
                dead_end: false,
            };

            let cache = Self::Cache {"
                join_ts!(0..nbr_elements, i, ti(i) ":" ident!("c" i) ",")
                "cplx: sum_cplx,
                vose_alias :" cm.VoseAlias "::new(vec!["
                    join_ts!(0..nbr_elements, i,
                        ident!("cplx_" i) "/ sum_cplx"
                    , separator: ",") "
                ])
            };

            " cm.Some "((cache, step))
        }
        
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
        
        fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {"
            join_ts!(0..nbr_elements, i,
                "let mut" ti_value(i) ":" cm.Option "<_> =" cm.None ";"
            )
            "let mut indices = ( 0 .." nbr_elements ").collect::<" cm.Vec "<_>>();"
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
                        ti_value(i) ".unwrap()"
                    , separator:",")
                    ")
                ),
                sum_cplx,
            )
        }
        
        fn ordered_mutate<'a>(
            &'a self,
            value: " tuple_mut ",
            cache: &'a mut Self::Cache,
            step: &'a mut Self::MutationStep,
            max_cplx: f64,
        ) -> " cm.Option "<(Self::UnmutateToken, f64)> {
            if max_cplx < <Self as" cm.TupleMutator "<T , " cm.TupleN_ident "<" tuple_type_params "> > >::min_complexity(self) { return " cm.None " }
            if step.inner.is_empty() || step.dead_end {
                return " cm.None ";
            }
            let step_idx = loop {
                let candidate = cache.vose_alias.sample();
                if ! unsafe { *step.dead_ends.get_unchecked(candidate) } { 
                    break candidate
                }
            };
            let current_cplx = " SelfAsTupleMutator "::complexity(self, " TupleNAsRefTypes "::get_ref_from_mut(&value), cache); 
            let inner_step_to_remove: usize;

            match step.inner[step_idx] {"
            join_ts!(0..nbr_elements, i,
                "InnerMutationStep::" Ti(i) "=> {
                    let old_field_cplx = self." mutator_i(i) ".complexity(value." i ", &cache." ti(i) ");
                    let max_field_cplx = max_cplx - current_cplx + old_field_cplx;
                    if let " cm.Some "((token, new_field_cplx)) =
                        self." mutator_i(i) "
                            .ordered_mutate(value." i ", &mut cache." ti(i) ", &mut step." ti(i) ", max_field_cplx)
                    {
                        return " cm.Some "((Self::UnmutateToken {
                            " ti(i) ": " cm.Some "(token),
                            ..Self::UnmutateToken::default()
                        }, current_cplx - old_field_cplx + new_field_cplx));
                    } else {
                        inner_step_to_remove = step_idx;
                    }
                }"
            )"
            }
            step.dead_ends[inner_step_to_remove] = true;
            if !step.dead_ends.contains(&false) {
                step.dead_end = true;
                return None
            }
            " SelfAsTupleMutator "::ordered_mutate(self, value, cache, step, max_cplx)
        }
        
        fn random_mutate<'a>(&'a self, value: " tuple_mut ", cache: &'a mut Self::Cache, max_cplx: f64, ) -> (Self::UnmutateToken, f64) {
            let current_cplx = " SelfAsTupleMutator "::complexity(self, " TupleNAsRefTypes "::get_ref_from_mut(&value), cache);
            match cache.vose_alias.sample() {"
                join_ts!(0..nbr_elements, i,
                    i "=> {
                        let old_field_cplx = self." mutator_i(i) ".complexity(value." i ", &cache." ti(i) ");
                        let max_field_cplx = max_cplx - current_cplx + old_field_cplx;
                        let (token, new_field_cplx) = self." mutator_i(i) "
                            .random_mutate(value." i ", &mut cache." ti(i) ", max_field_cplx) ;
                        
                        return (Self::UnmutateToken {
                            " ti(i) ": " cm.Some "(token),
                            ..Self::UnmutateToken::default()
                        },  current_cplx - old_field_cplx + new_field_cplx);
                    }"
                )
                "_ => unreachable!() ,
            }
        }
        
        fn unmutate<'a>(&'a self, value: " tuple_mut ", cache: &'a mut Self::Cache, t: Self::UnmutateToken) {"
            join_ts!(0..nbr_elements, i,
                "if let" cm.Some "(subtoken) = t." ti(i) "{
                    self. " mutator_i(i) ".unmutate(value." i ", &mut cache." ti(i) ", subtoken);
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
    use decent_synquote_alternative::{parser::TokenParser, token_builder};
    use proc_macro2::TokenStream;
    use token_builder::TokenBuilder;

    use super::{
        declare_tuple_mutator, declare_tuple_mutator_helper_types, impl_default_mutator_for_struct,
        impl_default_mutator_for_tuple, impl_mutator_trait, impl_tuple_structure_trait, make_tuple_type_structure,
    };

    #[test]
    fn test_impl_default_mutator_one_field() {
        let mut tb = TokenBuilder::new();
        let code = "
        pub struct Y {
            x: bool,
        }        
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let struc = parser.eat_struct().unwrap();
        impl_tuple_structure_trait(&mut tb, &struc);
        // impl_default_mutator_for_struct(&mut tb, &struc, &<_>::default());
        let generated = tb.end().to_string();

        let expected = "".parse::<TokenStream>().unwrap().to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}\n\n", generated, expected);
    }

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
        impl_default_mutator_for_struct(&mut tb, &struc, &<_>::default());
        let generated = tb.end().to_string();

        let inner_mutator = "fuzzcheck_mutators :: TupleMutatorWrapper < S < T > , fuzzcheck_mutators :: Tuple2Mutator < u8 , Vec < T > , M0 , M1 >, fuzzcheck_mutators :: Tuple2 < u8 , Vec < T > > >";

        let expected = format!("
pub struct SMutator < T : Into < u8 >, M0 , M1 > where 
    T : Default , 
    T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    pub mutator : {inner_mutator} 
}} 
pub struct SMutatorCache < T : Into < u8 >, M0 , M1 > where 
    T : Default , 
    T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    inner : < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: Cache , 
}} 
impl < T : Into < u8 >, M0 , M1 > SMutatorCache < T , M0 , M1 > where 
    T : Default , 
    T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    fn new (inner : < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: Cache) -> Self 
    {{ 
        Self {{ inner : (inner) }} 
    }} 
}} 
impl < T : Into < u8 >, M0 , M1 > :: std :: clone :: Clone for SMutatorCache < T , M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    fn clone (& self) -> Self {{ 
        Self {{ 
            inner : self . inner . clone () 
        }} 
    }} 
}} 
pub struct SMutatorMutationStep < T : Into < u8 >, M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    inner : < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: MutationStep , 
}} 
impl < T : Into < u8 >, M0 , M1 > SMutatorMutationStep < T , M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    fn new (inner : < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: MutationStep) -> Self 
    {{ 
        Self {{ inner : (inner) }} 
    }} 
}} 
impl < T : Into < u8 >, M0 , M1 > :: std :: clone :: Clone for SMutatorMutationStep < T , M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static ,
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    fn clone (& self) -> Self {{ 
        Self {{ inner : self . inner . clone () }} 
    }} 
}} 
pub struct SMutatorArbitraryStep < T : Into < u8 >, M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static ,
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    inner : < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: ArbitraryStep , 
}} 
impl < T : Into < u8 >, M0 , M1 > SMutatorArbitraryStep < T , M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    fn new (inner : < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: ArbitraryStep) -> Self 
    {{ 
        Self {{ inner : (inner) }} 
    }} 
}} 
impl < T : Into < u8 >, M0 , M1 > :: std :: clone :: Clone for SMutatorArbitraryStep < T , M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    fn clone (& self) -> Self {{ 
        Self {{ inner : self . inner . clone () }} 
    }} 
}} 
pub struct SMutatorUnmutateToken < T : Into < u8 >, M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    inner : < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: UnmutateToken , 
}} 
impl < T : Into < u8 >, M0 , M1 > SMutatorUnmutateToken < T , M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    fn new (inner : < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > >:: UnmutateToken) -> Self 
    {{ 
        Self {{ inner : (inner) }} 
    }} 
}} 
impl < T : Into < u8 >, M0 , M1 > SMutator < T , M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    pub fn new (mutator_x : M0 , mutator_y : M1) -> Self {{ 
        Self {{ 
            mutator : fuzzcheck_mutators :: TupleMutatorWrapper :: new (
                fuzzcheck_mutators :: Tuple2Mutator :: new (
                    mutator_x , 
                    mutator_y
                )
            ) 
        }} 
    }} 
}} 
impl < T : Into < u8 >, M0 , M1 > :: std :: default :: Default for SMutator < T , M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > , 
    M0 : :: std :: default :: Default , M1 : :: std :: default :: Default 
{{
     fn default () -> Self {{ 
        Self {{ 
             mutator : < _ >:: default () 
        }} 
    }} 
}} 
impl < T : Into < u8 >, M0 , M1 > fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > 
    for SMutator < T , M0 , M1 > 
    where T : Default , T : :: std :: clone :: Clone + 'static , 
    M0 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < u8 > , 
    M1 : fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Vec < T > > 
{{ 
    type Cache = SMutatorCache < T , M0 , M1 > ; 
    type MutationStep = SMutatorMutationStep < T , M0 , M1 > ; 
    type ArbitraryStep = SMutatorArbitraryStep < T , M0 , M1 > ; 
    type UnmutateToken = SMutatorUnmutateToken < T , M0 , M1 > ; 
    
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {{
        Self::ArbitraryStep::new(< {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > > ::default_arbitrary_step(&self.mutator))
    }}

    fn validate_value(&self, value: &S<T>) -> ::std::option::Option<(Self::Cache, Self::MutationStep)> {{
        if let ::std::option::Option::Some((c, s)) = < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > > :: validate_value (
            & self . mutator , 
            value
        ) {{
            ::std::option::Option::Some( (Self::Cache::new(c) , Self::MutationStep::new(s) ) )
        }} else {{
            ::std::option::Option::None
        }}
    }}

    fn max_complexity (& self) -> f64 {{ 
        < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > > :: max_complexity (& self . mutator) 
    }} 
    fn min_complexity (& self) -> f64 {{ 
        < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > > :: min_complexity (& self . mutator) 
    }} 
    fn complexity (& self , value : & S < T > , cache : & Self :: Cache) -> f64 {{ 
        < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > > :: complexity (& self . mutator , value , & cache . inner) 
    }} 
    fn ordered_arbitrary (& self , step : & mut Self :: ArbitraryStep , max_cplx : f64) -> Option < (S < T > , Self :: Cache) > {{ 
        if let :: std :: option :: Option :: Some ((value , cache)) = < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > > :: ordered_arbitrary (& self . mutator , & mut step . inner , max_cplx) {{ 
            :: std :: option :: Option :: Some ((value , Self :: Cache :: new (cache))) 
        }} else {{ 
            :: std :: option :: Option :: None 
        }} 
    }} 
    fn random_arbitrary (& self , max_cplx : f64) -> (S < T > , Self :: Cache) {{ 
        let (value , cache) = < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > > :: random_arbitrary (& self . mutator , max_cplx) ; 
        (value , Self :: Cache :: new (cache)) 
    }} 
    fn ordered_mutate (& self , value : & mut S < T > , cache : & mut Self :: Cache , step : & mut Self :: MutationStep , max_cplx : f64 ,) -> Option < Self :: UnmutateToken > {{ 
        if let :: std :: option :: Option :: Some (t) = < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > > :: ordered_mutate (& self . mutator , value , & mut cache . inner , & mut step . inner , max_cplx ,) {{ 
            :: std :: option :: Option :: Some (Self :: UnmutateToken :: new (t)) 
        }} else {{ 
            :: std :: option :: Option :: None 
        }} 
    }} 
    fn random_mutate (& self , value : & mut S < T > , cache : & mut Self :: Cache , max_cplx : f64) -> Self :: UnmutateToken {{ 
        Self :: UnmutateToken :: new (< {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > > :: random_mutate (& self . mutator , value , & mut cache . inner , max_cplx)) 
    }} 
    fn unmutate (& self , value : & mut S < T > , cache : & mut Self :: Cache , t : Self :: UnmutateToken) {{ 
        < {inner_mutator} as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < S < T > > > :: unmutate (& self . mutator , value , & mut cache . inner , t . inner) 
    }} 
}} 
impl < T : Into < u8 >> fuzzcheck_mutators :: DefaultMutator for S < T > 
    where T : Default , T : fuzzcheck_mutators :: DefaultMutator + 'static 
{{ 
    type Mutator = SMutator < T , < u8 as fuzzcheck_mutators :: DefaultMutator >:: Mutator , < Vec < T > as fuzzcheck_mutators :: DefaultMutator >:: Mutator > ; 
    
    fn default_mutator () -> Self :: Mutator {{ 
        SMutator :: new (
            < u8 as fuzzcheck_mutators :: DefaultMutator >:: default_mutator () , 
            < Vec < T > as fuzzcheck_mutators :: DefaultMutator >:: default_mutator ()
        ) 
    }} 
}} 
        ", inner_mutator = inner_mutator)
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
    
    fn default_arbitrary_step (& self) -> Self :: ArbitraryStep { 
        Self :: ArbitraryStep { 
            t0 : self . mutator_0 . default_arbitrary_step () , 
            t1 : self . mutator_1 . default_arbitrary_step () 
        } 
    }

    fn max_complexity(&self) -> f64 {
        self.mutator_0.max_complexity() + self.mutator_1.max_complexity()
    }
    fn min_complexity(&self) -> f64 {
        self.mutator_0.min_complexity() + self.mutator_1.min_complexity()
    }
    fn complexity<'a>(&'a self, _value: (&'a T0, &'a T1), cache: &'a Self::Cache) -> f64 {
        cache.cplx
    }

    fn validate_value<'a>(&'a self, value: (&'a T0, &'a T1)) -> ::std::option::Option<(Self::Cache, Self::MutationStep)> {
        let (c0, s0) = self.mutator_0.validate_value(value.0)?;
        let (c1, s1) = self.mutator_1.validate_value(value.1)?;
        let cplx = self.mutator_0.complexity(value.0, &c0) + self.mutator_1.complexity(value.1, &c1);

        let step = 0;
        let step = Self::MutationStep {
            t0: s0,
            t1: s1,
            inner: vec![InnerMutationStep::T0, InnerMutationStep::T1],
            step,
        };
        let cache = Self::Cache { t0: c0, t1: c1, cplx };

        ::std::option::Option::Some((cache, step))
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
        let mut t0_step: ::std::option::Option<_> = ::std::option::Option::None;
        let mut t1_value: ::std::option::Option<_> = ::std::option::Option::None;
        let mut t1_cache: ::std::option::Option<_> = ::std::option::Option::None;
        let mut t1_step: ::std::option::Option<_> = ::std::option::Option::None;
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
                    t0_step = ::std::option::Option::Some(step);
                }
                1 => {
                    let (value, cache) = self.mutator_1.random_arbitrary(max_cplx - cplx);
                    cplx += self.mutator_1.complexity(&value, &cache);
                    t1_value = ::std::option::Option::Some(value);
                    t1_cache = ::std::option::Option::Some(cache);
                    t1_step = ::std::option::Option::Some(step);
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
            }
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
pub struct Cache<T0, T1> {
    t0: T0,
    t1: T1,
    cplx: f64
}
pub enum InnerMutationStep {
    T0,
    T1
}
pub struct MutationStep<T0, T1> {
    t0: T0,
    t1: T1,
    step: usize,
    inner: ::std::vec::Vec<InnerMutationStep>
}

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

    #[test]
    fn test_make_basic_tuple_mutator() {
        let mut tb = TokenBuilder::new();
        super::make_basic_tuple_mutator(&mut tb, 2);
        let generated = tb.end();
        eprintln!("\n\n{}\n\n", generated);
        panic!();
    }
}
