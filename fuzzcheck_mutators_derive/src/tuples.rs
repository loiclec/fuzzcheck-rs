use decent_synquote_alternative as synquote;
use proc_macro2::{Ident, Span, TokenStream};

use synquote::parser::*;
use synquote::token_builder::*;

pub fn make_basic_tuple_mutator(tb: &mut TokenBuilder, nbr_elements: usize, fuzzcheck_mutators_crate: TokenStream) {
    make_tuple_type_structure(tb, nbr_elements, fuzzcheck_mutators_crate.clone());
    let mod_mutator = ident!("tuple" nbr_elements) ;
    extend_ts!(tb,
        "pub use" mod_mutator " :: " ident!("Tuple" nbr_elements "Mutator") ";"
        "mod" mod_mutator "{"
            "use super:: " ident!("Tuple" nbr_elements) " ;"
    );

    declare_tuple_mutator(tb, nbr_elements);
    declare_tuple_mutator_helper_types(tb, nbr_elements);
    impl_mutator_trait(tb, nbr_elements, fuzzcheck_mutators_crate.clone());

    impl_default_mutator_for_tuple(tb, nbr_elements, fuzzcheck_mutators_crate);

    extend_ts!(tb,
        "}"
    );
}

pub fn make_tuple_type_structure(tb: &mut TokenBuilder, nbr_elements: usize, fuzzcheck_mutators_crate: TokenStream) {
    let Ti = |i: usize| ident!("T" i);
    // T0, T1, ...
    let type_params = join_ts!(0..nbr_elements, i, Ti(i), separator: ",");
    let type_params_static_bound = join_ts!(0..nbr_elements, i, Ti(i) ": 'static", separator: ",");
    let tuple_owned = ts!("(" type_params ")");
    let tuple_ref = ts!("(" join_ts!(0..nbr_elements, i, "&'a" Ti(i), separator: ",") ")");
    let tuple_mut = ts!("(" join_ts!(0..nbr_elements, i, "&'a mut" Ti(i), separator: "," ) ")");

    let TupleN = ident!("Tuple" nbr_elements);
    let PhantomData = ts!("::std::marker::PhantomData<(" type_params ")>");
    let RefTypes = ts!(fuzzcheck_mutators_crate "::RefTypes");

    extend_ts!(tb,
        "pub struct" TupleN "<" type_params_static_bound "> {
            _phantom: " PhantomData ",
        }
        impl<" type_params_static_bound "> " RefTypes " for " TupleN "<" type_params "> {
            type Owned = " tuple_owned ";
            type Ref<'a> = " tuple_ref ";
            type Mut<'a> = " tuple_mut ";
            fn get_ref_from_mut<'a>(v: &'a Self::Mut<'a>) -> Self::Ref<'a> {
                (" join_ts!(0..nbr_elements, i, "v." i, separator: ",") ")
            }
        }
        "
        "impl<" type_params_static_bound "> TupleStructure<" TupleN "<" type_params "> > for" tuple_owned "{
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

pub fn impl_wrapped_tuple_1_structure(tb: &mut TokenBuilder, struc: &Struct, fuzzcheck_mutators_crate: TokenStream) {
    assert!(struc.struct_fields.len() == 1);
    let field = &struc.struct_fields[0];
    let field_type = field.ty.clone();

    let generics_no_eq = struc.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = struc.generics.removing_bounds_and_eq_type();
    
    let TupleStructure = ts!(fuzzcheck_mutators_crate "::TupleStructure");
    let WrappedTuple1 = ts!(fuzzcheck_mutators_crate "::WrappedTuple1");

    let mut where_clause = struc.where_clause.clone().unwrap_or(WhereClause::default());
    where_clause.items.push(
        WhereClauseItem {
            for_lifetimes: None,
            lhs: field_type.clone(),
            rhs: ts!("'static"),
        }
    );

    extend_ts!(tb,
        "impl " generics_no_eq TupleStructure "<" WrappedTuple1 "<" field_type "> > 
            for " struc.ident generics_no_eq_nor_bounds where_clause " 
        {
            fn get_ref<'a>(&'a self) -> &'a " field_type " {
                &self." field.access() "
            }
        
            fn get_mut<'a>(&'a mut self) -> &'a mut " field_type " {
                &mut self." field.access() "
            }
        
            fn new(t: " field_type ") -> Self {
                Self { " field.access() ": t }
            }
        }"
    )
}

pub fn impl_tuple_structure_trait(tb: &mut TokenBuilder, struc: &Struct, fuzzcheck_mutators_crate: TokenStream) {
    let nbr_elements = struc.struct_fields.len();

    let field_types = join_ts!(&struc.struct_fields, field, field.ty, separator: ",");
    // let Ti = |i: usize| ident!("T" i);
    let TupleStructure = ts!(fuzzcheck_mutators_crate "::TupleStructure");
    let TupleN = ts!(fuzzcheck_mutators_crate "::" ident!("Tuple" nbr_elements));
    let generics_no_eq = struc.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = struc.generics.removing_bounds_and_eq_type();

    let tuple_owned = ts!("(" join_ts!(&struc.struct_fields, field, field.ty , separator: ",") ")");
    let tuple_ref = ts!("(" join_ts!(&struc.struct_fields, field, "&'a" field.ty , separator: ",") ")");
    let tuple_mut = ts!("(" join_ts!(&struc.struct_fields, field, "&'a mut" field.ty , separator: ",") ")");

    let mut where_clause = struc.where_clause.clone().unwrap_or_default();
    for tp in &struc.generics.type_params {
        where_clause.items.push(WhereClauseItem {
            for_lifetimes: None,
            lhs: tp.type_ident.clone(),
            rhs: ts!("'static"),
        });
    }

    extend_ts!(tb,
        "impl" generics_no_eq TupleStructure "<" TupleN "<" field_types "> > 
            for" struc.ident generics_no_eq_nor_bounds where_clause "{
            fn get_ref<'a>(&'a self) -> " tuple_ref " {
                (" join_ts!(&struc.struct_fields, field, "&self." field.access(), separator: ",") ")
            }

            fn get_mut<'a>(&'a mut self) -> " tuple_mut " {
                (" join_ts!(&struc.struct_fields, field, "&mut self." field.access(), separator: ",") ")
            }

            fn new(t:" tuple_owned ") -> Self {
                Self {"
                    join_ts!(struc.struct_fields.iter().enumerate(), (i, field),
                        field.access() ": t." i 
                    , separator: ",")
                "}
            }
        }"
    );
}

pub fn impl_default_mutator_for_struct_with_0_field(tb: &mut TokenBuilder, struc: &Struct, fuzzcheck_mutators_crate: TokenStream) {
    assert!(struc.struct_fields.len() == 0);

    let generics_no_eq = struc.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = struc.generics.removing_bounds_and_eq_type();

    let DefaultMutator = ts!(fuzzcheck_mutators_crate "::DefaultMutator");
    let UnitMutator = ts!(fuzzcheck_mutators_crate "::UnitMutator");

    // add T: DefaultMutator for each generic type parameter to the existing where clause
    let mut where_clause = struc.where_clause.clone().unwrap_or(WhereClause::default());
    for ty_param in struc.generics.type_params.iter() {
        where_clause.items.push(WhereClauseItem {
            for_lifetimes: None,
            lhs: ty_param.type_ident.clone(),
            rhs: DefaultMutator.clone(),
        });
    }

    let init = struc.kind.map(|kind| ts!(kind.open() kind.close()));

    extend_ts!(tb, 
    "impl " generics_no_eq DefaultMutator "for" struc.ident generics_no_eq_nor_bounds where_clause "{
        type Mutator = " UnitMutator "<Self>;
    
        fn default_mutator() -> Self::Mutator {
            Self::Mutator::new(" struc.ident init ")
        }
    }
    ");

}

pub fn impl_default_mutator_for_struct_with_1_field(tb: &mut TokenBuilder, struc: &Struct, fuzzcheck_mutators_crate: TokenStream) {
    assert!(struc.struct_fields.len() == 1);

    let field = &struc.struct_fields[0];

    let generics_no_eq = struc.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = struc.generics.removing_bounds_and_eq_type();

    let DefaultMutator = ts!(fuzzcheck_mutators_crate "::DefaultMutator");
    let WrappedMutator = ts!(fuzzcheck_mutators_crate "::WrappedMutator");

    // add T: DefaultMutator for each generic type parameter to the existing where clause
    let mut where_clause = struc.where_clause.clone().unwrap_or(WhereClause::default());
    for ty_param in struc.generics.type_params.iter() {
        where_clause.items.push(WhereClauseItem {
            for_lifetimes: None,
            lhs: ty_param.type_ident.clone(),
            rhs: DefaultMutator.clone(),
        });
    }

    extend_ts!(tb, 
    "impl " generics_no_eq DefaultMutator "for" struc.ident generics_no_eq_nor_bounds where_clause "{
        type Mutator = " WrappedMutator "<" field.ty ", <" field.ty "as" DefaultMutator ">::Mutator>;
    
        fn default_mutator() -> Self::Mutator {
            Self::Mutator::new(<" field.ty ">::default_mutator())
        }
    }
    ")
}


pub fn impl_default_mutator_for_struct_with_more_than_1_field(tb: &mut TokenBuilder, struc: &Struct, fuzzcheck_mutators_crate: TokenStream) {
    assert!(struc.struct_fields.len() > 1);

    let nbr_elements = struc.struct_fields.len();
    assert!(nbr_elements > 1);

    let TupleNMutator = ts!(fuzzcheck_mutators_crate "::" ident!("Tuple" nbr_elements "Mutator"));
    let generics_no_eq = struc.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = struc.generics.removing_bounds_and_eq_type();

    let DefaultMutator = ts!(fuzzcheck_mutators_crate "::DefaultMutator");

    let field_types = join_ts!(&struc.struct_fields, field, field.ty, separator: ",");
    
    let TupleN = ts!(fuzzcheck_mutators_crate "::" ident!("Tuple" nbr_elements) "<" field_types ">");

    let TupleMutatorWrapper = ts!(
        fuzzcheck_mutators_crate "::TupleMutatorWrapper<"
            struc.ident generics_no_eq_nor_bounds "," 
            TupleNMutator "<" 
                field_types ", "
                join_ts!(&struc.struct_fields, field,
                    "<" field.ty "as" DefaultMutator "> :: Mutator"
                , separator: ",")
            ">," 
            TupleN 
        ">"
    );

    // add T: DefaultMutator for each generic type parameter to the existing where clause
    let mut where_clause = struc.where_clause.clone().unwrap_or(WhereClause::default());
    for ty_param in struc.generics.type_params.iter() {
        where_clause.items.push(WhereClauseItem {
            for_lifetimes: None,
            lhs: ty_param.type_ident.clone(),
            rhs: ts!(DefaultMutator "+ 'static"),
        });
    }

    extend_ts!(tb,
    "impl" generics_no_eq DefaultMutator "for" struc.ident generics_no_eq_nor_bounds where_clause "{"
        "type Mutator = " TupleMutatorWrapper ";
        
        fn default_mutator() -> Self::Mutator {
            Self::Mutator::new(" TupleNMutator "::new("
            join_ts!(&struc.struct_fields, field,
                "< " field.ty " >::default_mutator() ,"
            )
            "))
        }
        "
    "}"
    )
}

fn declare_tuple_mutator(tb: &mut TokenBuilder, nbr_elements: usize) {
    let clone = ts!("::std::clone::Clone");
    let mutator = ts!("::fuzzcheck_traits::Mutator");
    let rng = ts!("::fastrand::Rng");
    let phantom_data = ts!("::std::marker::PhantomData");

    let name_mutator = ident!("Tuple" nbr_elements "Mutator");

    let tuple_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");
    let mutator_type_params = join_ts!(0..nbr_elements, i, ident!("M" i), separator: ",");
    let type_params = ts!(tuple_type_params "," mutator_type_params);
    let tuple_type = ts!("(" tuple_type_params ")");

    let where_clause = ts!(
        "where"
        join_ts!(0..nbr_elements, i,
            ident!("T" i) ":" clone ","
            ident!("M" i) ":" mutator "<" ident!("T" i) ">"
        ,separator: ",")
    );

    let mutator_type_params_replacing_one_by_m = |replacing: usize| -> TokenStream {
        join_ts!(0..nbr_elements, i, 
            if i == replacing {
                ident!("M")
            } else {
                ident!("M" i)
            }
        , separator: ",")
    };

    extend_ts!(tb,
        "pub struct" name_mutator "<" type_params ">" where_clause
        "{"
            join_ts!(0..nbr_elements, i,
                "pub" ident!("mutator_" i) ":" ident!("M" i) ","
            )
            "rng :" rng ",
            _phantom :" phantom_data "<" tuple_type ">"
        "}
        
        impl < " type_params " >" name_mutator "<" type_params ">" where_clause "{
            pub fn new(" join_ts!(0..nbr_elements, i, ident!("mutator_" i) ":" ident!("M" i), separator: ",") ") -> Self {
                Self {"
                    join_ts!(0..nbr_elements, i,
                        ident!("mutator_" i) ","
                    )
                    "rng: <_>::default() ,
                    _phantom:" phantom_data
                "}
            }"
            join_ts!(0..nbr_elements, i,
                "pub fn" ident!("replacing_mutator_" i) " < M > ( self , mutator : M )
                    ->" name_mutator "<" tuple_type_params ", " mutator_type_params_replacing_one_by_m(i) " >" "
                    where M :" mutator "<" ident!("T" i) "> 
                {
                    " name_mutator " {"
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

fn declare_tuple_mutator_helper_types(tb: &mut TokenBuilder, nbr_elements: usize) {
    let clone = ts!("::std::clone::Clone");
    let default = ts!("::std::default::Default");
    let option = ts!("::std::option::Option");
    let none = ts!(option "::None");
    let vec = ts!("::std::vec::Vec");
    let tuple_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");

    extend_ts!(tb,
        "#[derive( " clone " )]
        pub struct Cache <" tuple_type_params "> {"
            join_ts!(0..nbr_elements, i,
                ident!("t" i) ":" ident!("T" i) ","
            )
            "cplx : f64
        }
        #[derive( " clone " )]
        pub enum InnerMutationStep {"
            join_ts!(0..nbr_elements, i,
                ident!("T" i)
            , separator: ",")
        "}
        #[derive( " clone " )]
        pub struct MutationStep < " tuple_type_params " > {"
            join_ts!(0..nbr_elements, i,
                ident!("t" i) ":" ident!("T" i) ","
            )
            "step : usize ,
            inner : " vec " < InnerMutationStep > 
        }
        #[derive(" default "," clone ")]
        pub struct ArbitraryStep < " tuple_type_params " > {"
            join_ts!(0..nbr_elements, i,
                ident!("t" i) ":" ident!("T" i)
            , separator: ",")
        "}

        pub struct UnmutateToken < " tuple_type_params " > {"
            join_ts!(0..nbr_elements, i,
                ident!("t" i) ":" option "<" ident!("T" i) "> ,"
            )
            "cplx : f64
        }
        impl < " tuple_type_params " > " default " for UnmutateToken < " tuple_type_params " > {
            fn default() -> Self {
                Self {"
                    join_ts!(0..nbr_elements, i,
                        ident!("t" i) ":" none ","
                    )
                    "cplx : <_>::default()
                }
            }
        }
        "
    )
}

fn impl_mutator_trait(tb: &mut TokenBuilder, nbr_elements: usize, fuzzcheck_mutators_crate: TokenStream) {
    let clone = ts!("::std::clone::Clone");
    //let default = ts!("::std::default::Default");
    let option = ts!("::std::option::Option");
    let some = ts!(option "::Some");
    let none = ts!(option "::None");
    let vec = ts!("::std::vec::Vec");
    let tuple_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");

    let TupleStructure = ts!(fuzzcheck_mutators_crate "::TupleStructure");
    let TupleMutator = ts!(fuzzcheck_mutators_crate "::TupleMutator");
    let Mutator = ts!("::fuzzcheck_traits::Mutator");
    let fastrand = ts!(fuzzcheck_mutators_crate "::fastrand");

    let TupleN = ident!("Tuple" nbr_elements);
    let TupleNMutator = ident!("Tuple" nbr_elements "Mutator");


    let mutator_type_params = join_ts!(0..nbr_elements, i, ident!("M" i), separator: ",");
    let type_params = ts!(tuple_type_params "," mutator_type_params);

    let ti = |i: usize| ident!("t" i);

    let Ti = |i: usize| ident!("T" i);

    let Mi = |i: usize| ident!("M" i);
    let mutator_i = |i: usize| ident!("mutator_" i);
    // let get_i = |i: usize| ts!(ident!("get_" i) "()" );
    // let get_i_mut = |i: usize| ts!(ident!("get_" i "_mut") "()");
    let ti_value = |i: usize| ident!("t" i "_value");
    let ti_cache = |i: usize| ident!("t" i "_cache");
    // let get_ = |i: usize| ident!("get_" i);

    // let tuple_owned = ts!("(" join_ts!(0..nbr_elements, i, Ti(i), separator: ",") ")");
    let tuple_ref = ts!("(" join_ts!(0..nbr_elements, i, "&'a" Ti(i), separator: ",") ")");
    let tuple_mut = ts!("(" join_ts!(0..nbr_elements, i, "&'a mut" Ti(i), separator: ",") ")");

    let SelfAsTupleMutator = ts!("<Self as " TupleMutator "<T, " TupleN "<" tuple_type_params "> >>");
    let RefTypes = ts!(fuzzcheck_mutators_crate "::RefTypes");
    let TupleNAsRefTypes = ts!("<" TupleN "<" tuple_type_params "> as " RefTypes ">");

    extend_ts!(tb,"
    impl <T , " type_params " > " TupleMutator "<T , " TupleN "<" tuple_type_params "> > 
        for " TupleNMutator "< " type_params " >
    where
        T: " clone "," 
        join_ts!(0..nbr_elements, i,
            Ti(i) ":" clone " + 'static ,"
            Mi(i) ":" Mutator "<" Ti(i) ">,"
        ) "
        T: " TupleStructure "<" TupleN "<" tuple_type_params "> >,
    {
        type Cache = Cache <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" Mutator "<" Ti(i) "> >::Cache "
            , separator: ",")
        ">;
        type MutationStep = MutationStep <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" Mutator "<" Ti(i) "> >::MutationStep "
            , separator: ",")
        ">;
        type ArbitraryStep = ArbitraryStep <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" Mutator "<" Ti(i) "> >::ArbitraryStep "
            , separator: ",")
        ">;
        type UnmutateToken = UnmutateToken <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" Mutator "<" Ti(i) "> >::UnmutateToken "
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
            &mut self,
            step: &mut Self::ArbitraryStep,
            max_cplx: f64,
        ) -> " option "<(T, Self::Cache)> {
            " // TODO! actually write something that is ordered_arbitrary sense here
            some "  (self.random_arbitrary(max_cplx))
        }
        fn random_arbitrary(&mut self, max_cplx: f64) -> (T, Self::Cache) {"
            join_ts!(0..nbr_elements, i,
                "let mut" ti_value(i) ":" option "<_> =" none ";"
                "let mut" ti_cache(i) ":" option "<_> =" none ";"
            )
            "let mut indices = ( 0 .." nbr_elements ").collect::<" vec "<_>>();"
            fastrand "::shuffle(&mut indices);"
            "let mut cplx = 0.0;
            for idx in indices.iter() {
                match idx {"
                join_ts!(0..nbr_elements, i,
                    i "=> {
                        let (value, cache) = self." mutator_i(i) ".random_arbitrary(max_cplx - cplx);
                        cplx += self." mutator_i(i) ".complexity(&value, &cache);
                        " ti_value(i) "= " some "(value);
                        " ti_cache(i) "= " some "(cache);
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
            &'a mut self,
            value: " tuple_mut ",
            cache: &'a mut Self::Cache,
            step: &'a mut Self::MutationStep,
            max_cplx: f64,
        ) -> " option "<Self::UnmutateToken> {
            if step.inner.is_empty() {
                return " none ";
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
                    if let " some "(token) =
                        self." mutator_i(i) "
                            .ordered_mutate(value." i ", &mut cache." ti(i) ", &mut step." ti(i) ", max_field_cplx)
                    {
                        let new_field_complexity = self." mutator_i(i) ".complexity(value." i ", &cache." ti(i) ");
                        cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                        return " some "(Self::UnmutateToken {
                            " ti(i) ": " some "(token),
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
        fn random_mutate<'a>(&'a mut self, value: " tuple_mut ", cache: &'a mut Self::Cache, max_cplx: f64, ) -> Self::UnmutateToken {
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
                            " ti(i) ": " some "(token),
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
                "if let" some "(subtoken) = t." ti(i) "{
                    self. " mutator_i(i) ".unmutate(value." i ", &mut cache. " ti(i) " , subtoken);
                }"
            )
        "}
    }
    "
    )
}

fn impl_default_mutator_for_tuple(tb: &mut TokenBuilder, nbr_elements: usize, fuzzcheck_mutators_crate: TokenStream) {
    let tuple_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");
    let mutator_type_params = join_ts!(0..nbr_elements, i, ident!("M" i), separator: ",");
    let type_params = ts!(tuple_type_params "," mutator_type_params);


    let Ti = |i: usize| ident!("T" i);
    let Mi = |i: usize| ident!("M" i);
    let DefaultMutator = ts!(fuzzcheck_mutators_crate "::DefaultMutator");
    let Default = ts!("::std::default::Default");
    let Clone = ts!("::std::clone::Clone");
    let Mutator = ts!("::fuzzcheck_traits::Mutator");

    let TupleN = ts!(ident!("Tuple" nbr_elements) "<" tuple_type_params ">");
    let TupleNMutator = ident!("Tuple" nbr_elements "Mutator");
    let TupleMutatorWrapper = ts!(
        fuzzcheck_mutators_crate "::TupleMutatorWrapper<
            (" tuple_type_params ")," 
            TupleNMutator "<" 
                tuple_type_params ", "
                join_ts!(0..nbr_elements, i,
                    "<" Ti(i) "as" DefaultMutator "> :: Mutator"
                , separator: ",")
            ">," 
            TupleN 
        ">"
    );


    extend_ts!(tb,    
    "
    impl<" type_params ">" Default "for" TupleNMutator "<" type_params ">
        where"
        join_ts!(0..nbr_elements, i,
            Ti(i) ":" Clone ","
            Mi(i) ":" Mutator "<" Ti(i) ">,"
        )
        join_ts!(0..nbr_elements, i, Mi(i) ":" Default, separator: ",")
    "{
        fn default() -> Self {
            Self::new("
                join_ts!(0..nbr_elements, i, 
                    "<" Mi(i) "as" Default "> :: default()"
                , separator: ",")
            ")
        }
    } 

    impl<" tuple_type_params ">" DefaultMutator "for (" tuple_type_params ")
        where" join_ts!(0..nbr_elements, i, Ti(i) ":" DefaultMutator "+ 'static", separator: ",")
    "{
        type Mutator = " TupleMutatorWrapper ";
        fn default_mutator() -> Self::Mutator {
            Self::Mutator::new(" TupleNMutator "::new("
                join_ts!(0..nbr_elements, i, 
                    "<" Ti(i) "as" DefaultMutator "> :: default_mutator()"
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
    use decent_synquote_alternative::TokenBuilderExtend;

    use super::{declare_tuple_mutator, declare_tuple_mutator_helper_types, impl_default_mutator_for_struct_with_1_field, impl_default_mutator_for_struct_with_more_than_1_field, impl_default_mutator_for_tuple, impl_mutator_trait, impl_tuple_structure_trait, impl_wrapped_tuple_1_structure, make_tuple_type_structure};

    #[test]
    fn test_default_mutator_for_struct_with_1_field() {
        let mut tb = TokenBuilder::new();
        let code = "
        pub struct X<T: Clone>(bool) where T: Default;
        ".parse::<TokenStream>().unwrap();
        let mut parser = TokenParser::new(code);
        let struc = parser.eat_struct().unwrap();
        impl_default_mutator_for_struct_with_1_field(&mut tb, &struc, ts!("fuzzcheck_mutators"));
        let generated = tb.end().to_string();

        let expected = "
        impl<T: Clone> fuzzcheck_mutators::DefaultMutator for X <T> where T: Default, T: fuzzcheck_mutators::DefaultMutator {
            type Mutator = fuzzcheck_mutators::WrappedMutator<bool, <bool as fuzzcheck_mutators::DefaultMutator >::Mutator >;
        
            fn default_mutator() -> Self::Mutator {
                Self::Mutator::new(<bool>::default_mutator())
            }
        }
        ".parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_wrapped_tuple_1_structure() {
        let mut tb = TokenBuilder::new();
        let code = "
        pub struct X<T> where T: Default {
            x: Vec<T> ,
        }
        ".parse::<TokenStream>().unwrap();
        let mut parser = TokenParser::new(code);
        let struc = parser.eat_struct().unwrap();
        impl_wrapped_tuple_1_structure(&mut tb, &struc, ts!("fuzzcheck_mutators"));
        let generated = tb.end().to_string();

        let expected = "
        impl<T> fuzzcheck_mutators::TupleStructure<fuzzcheck_mutators::WrappedTuple1<Vec<T> > > for X<T> where T: Default, Vec<T> : 'static {
            fn get_ref<'a>(&'a self) -> &'a Vec<T> {
                &self.x
            }
        
            fn get_mut<'a>(&'a mut self) -> &'a mut Vec<T> {
                &mut self.x
            }
        
            fn new(t: Vec<T>) -> Self {
                Self { x: t }
            }
        }
        ".parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}", generated, expected);
    }

    #[test]
    fn test_impl_default_mutator_two_fields() {
        let mut tb = TokenBuilder::new();
        let code = "
        pub struct S<T> {
            x: u8,
            y: Vec<T> ,
        }
        ".parse::<TokenStream>().unwrap();
        let mut parser = TokenParser::new(code);
        let struc = parser.eat_struct().unwrap();
        impl_default_mutator_for_struct_with_more_than_1_field(&mut tb, &struc, ts!("fuzzcheck_mutators"));
        let generated = tb.end().to_string();

        let expected = "
impl<T> fuzzcheck_mutators::DefaultMutator for S<T>
where
    T: fuzzcheck_mutators::DefaultMutator + 'static
{
    type Mutator = fuzzcheck_mutators::TupleMutatorWrapper< 
        S<T> ,
        fuzzcheck_mutators::Tuple2Mutator<
            u8,
            Vec<T> ,
            <u8 as fuzzcheck_mutators::DefaultMutator> ::Mutator,
            <Vec<T> as fuzzcheck_mutators::DefaultMutator> ::Mutator
        >,
        fuzzcheck_mutators::Tuple2<u8, Vec<T> >
    > ;
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(fuzzcheck_mutators::Tuple2Mutator::new(
            <u8>::default_mutator(),
            <Vec<T> >::default_mutator(),
        ))
    }
}
        ".parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}", generated, expected);
    }

    #[test]
    fn test_impl_default_mutator_for_tuple() {
        let mut tb = TokenBuilder::new();
        impl_default_mutator_for_tuple(&mut tb, 2, ts!("crate"));
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
impl<T0, T1> crate::DefaultMutator for (T0, T1)
where
    T0: crate::DefaultMutator + 'static,
    T1: crate::DefaultMutator + 'static
{
    type Mutator = crate::TupleMutatorWrapper<
        (T0, T1),
        Tuple2Mutator<T0, T1, <T0 as crate::DefaultMutator> ::Mutator, <T1 as crate::DefaultMutator> ::Mutator>,
        Tuple2<T0, T1>
    > ;

    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(Tuple2Mutator::new(
            <T0 as crate::DefaultMutator> ::default_mutator(),
            <T1 as crate::DefaultMutator> ::default_mutator()
        ))
    }
}
        ".parse::<TokenStream>()
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
        ".parse::<TokenStream>().unwrap();
        let mut parser = TokenParser::new(code);
        let struc = parser.eat_struct().unwrap();

        let mut tb = TokenBuilder::new();
        
        impl_tuple_structure_trait(&mut tb, &struc,ts!("crate"));

        let generated = tb.end().to_string();

        let expected = "  
impl<T, U: Clone> crate::TupleStructure<crate::Tuple2< u8, Vec<(T, U)> > > 
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
        ".parse::<TokenStream>()
        .unwrap()
        .to_string();

        assert_eq!(generated, expected, "\n\n{} \n\n{}", generated, expected);
    }
    
    #[test]
    fn test_impl_mutator_trait() {
        let mut tb = TokenBuilder::new();
        impl_mutator_trait(&mut tb, 2, ts!("crate"));
        let generated = tb.end().to_string();

        let expected = "
impl<T, T0, T1, M0, M1> crate::TupleMutator<T, Tuple2<T0, T1> > for Tuple2Mutator<T0, T1, M0, M1>
    where
    T: ::std::clone::Clone,
    T0: ::std::clone::Clone + 'static,
    M0: ::fuzzcheck_traits::Mutator<T0>,
    T1: ::std::clone::Clone + 'static,
    M1: ::fuzzcheck_traits::Mutator<T1>,
    T: crate::TupleStructure<Tuple2<T0, T1> >,
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
        &mut self,
        step: &mut Self::ArbitraryStep,
        max_cplx: f64,
    ) -> ::std::option::Option<(T, Self::Cache)> {
        ::std::option::Option::Some(self.random_arbitrary(max_cplx))
    }
    fn random_arbitrary(&mut self, max_cplx: f64) -> (T, Self::Cache) {
        let mut t0_value: ::std::option::Option<_> = ::std::option::Option::None;
        let mut t0_cache: ::std::option::Option<_> = ::std::option::Option::None;
        let mut t1_value: ::std::option::Option<_> = ::std::option::Option::None;
        let mut t1_cache: ::std::option::Option<_> = ::std::option::Option::None;
        let mut indices = (0..2).collect::< ::std::vec::Vec<_>>();
        crate::fastrand::shuffle(&mut indices);

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
        &'a mut self,
        value: (&'a mut T0, &'a mut T1),
        cache: &'a mut Self::Cache,
        step: &'a mut Self::MutationStep,
        max_cplx: f64,
    ) -> ::std::option::Option<Self::UnmutateToken> {
        if step.inner.is_empty() {
            return ::std::option::Option::None;
        }
        let orig_step = step.step;
        step.step += 1;
        let current_cplx =
            <Self as crate::TupleMutator<T,Tuple2<T0, T1> >> ::complexity(self, <Tuple2<T0, T1> as crate::RefTypes> ::get_ref_from_mut(&value), cache);
        let inner_step_to_remove: usize;

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
        <Self as crate::TupleMutator<T,Tuple2<T0, T1> >> ::ordered_mutate(self, value, cache, step, max_cplx)
    }
    fn random_mutate<'a>(
        &'a mut self,
        value: (&'a mut T0, &'a mut T1),
        cache: &'a mut Self::Cache,
        max_cplx: f64,
    ) -> Self::UnmutateToken {
        let current_cplx =
            <Self as crate::TupleMutator<T,Tuple2<T0, T1> >> ::complexity(self, <Tuple2<T0, T1> as crate::RefTypes> ::get_ref_from_mut(&value), cache);
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
    rng: ::fastrand::Rng,
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
        make_tuple_type_structure(&mut tb, 2, ts!("crate"));
        let generated = tb.end().to_string();
        let expected = r#"
        pub struct Tuple2<T0: 'static, T1: 'static> {
            _phantom: ::std::marker::PhantomData<(T0, T1)> ,
        }
        impl<T0: 'static, T1: 'static> crate::RefTypes for Tuple2<T0, T1> {
            type Owned = (T0, T1);
            type Ref<'a> = (&'a T0, &'a T1);
            type Mut<'a> = (&'a mut T0, &'a mut T1);
            fn get_ref_from_mut<'a>(v: &'a Self::Mut<'a>) -> Self::Ref<'a> {
                (v.0, v.1)
            }
        }
        impl<T0: 'static, T1: 'static> TupleStructure<Tuple2<T0, T1> > for (T0, T1) {
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
