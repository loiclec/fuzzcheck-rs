use decent_synquote_alternative as synquote;
use proc_macro2::{Ident, Literal, Span, TokenStream};

use synquote::parser::*;
use synquote::token_builder::*;

pub fn make_basic_tuple_mutator_impl(item: TokenStream) -> TokenStream {
    let mut tb = TokenBuilder::new();

    let mut parser = TokenParser::new(item);
    if let Some(l) = parser.eat_literal() {
        if let Ok(nbr_elements) = l.to_string().parse::<usize>() {
            make_tuple_type_structure(&mut tb, nbr_elements);
            let mod_mutator = ident!("tuple" nbr_elements) ;
            extend_ts!(&mut tb,
                "pub use" mod_mutator " :: " ident!("Tuple" nbr_elements "Mutator") ";"
                "mod" mod_mutator "{"
                    "use super:: " ident!("Tuple" nbr_elements "Structure") " ;"
            );
    
            declare_tuple_mutator(&mut tb, nbr_elements);
            declare_tuple_mutator_helper_types(&mut tb, nbr_elements);
            impl_mutator_trait(&mut tb, nbr_elements);

            extend_ts!(&mut tb,
                "}"
            );

            return tb.end().into();
        }
    }
    panic!()
}

fn make_tuple_type_structure(tb: &mut TokenBuilder, nbr_elements: usize) {
    // T0, T1, ...
    let sequence_of_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");
    let tuple_type = ts!("(" sequence_of_type_params ")");

    // Tuple2Structure
    let name_trait = ident!("Tuple" nbr_elements "Structure");

    extend_ts!(tb,
        "pub trait" name_trait "{"
            join_ts!(0..nbr_elements, i,
                "type" ident!("T" i) ";
                fn" ident!("get_" i) "(&self) -> & Self::" ident!("T" i) " ;
                fn" ident!("get_" i "_mut") "(&mut self) -> &mut Self::" ident!("T" i) ";
                "
            )
            "fn new(t: (" join_ts!(0 .. nbr_elements, i, "Self::" ident!("T" i), separator: "," )" ) ) -> Self ;"
        "}
        
        impl <" sequence_of_type_params ">" name_trait " for " tuple_type " {"
            join_ts!(0..nbr_elements, i,
                "type" ident!("T" i)" = " ident!("T" i) " ;
                fn" ident!("get_" i) "(&self) -> & Self::" ident!("T" i) " {
                    & self." i "
                }
                fn" ident!("get_" i "_mut") "( & mut self ) -> &mut Self::" ident!("T" i) " {
                    &mut self." i "
                }
                "
            )
            "fn new(t: (" join_ts!(0 .. nbr_elements, i, "Self ::" ident!("T" i), separator: "," )" ) ) -> Self {
                t
            }"
        "}"
    );
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

fn impl_mutator_trait(tb: &mut TokenBuilder, nbr_elements: usize) {
    let clone = ts!("::std::clone::Clone");
    //let default = ts!("::std::default::Default");
    let option = ts!("::std::option::Option");
    let some = ts!(option "::Some");
    let none = ts!(option "::None");
    let vec = ts!("::std::vec::Vec");
    let tuple_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");

    let mutator = ts!("::fuzzcheck_traits::Mutator");
    let fastrand = ts!("::fastrand");

    let name_mutator = ident!("Tuple" nbr_elements "Mutator");
    let tuple_structure = ident!("Tuple" nbr_elements "Structure");

    let mutator_type_params = join_ts!(0..nbr_elements, i, ident!("M" i), separator: ",");
    let type_params = ts!(tuple_type_params "," mutator_type_params);

    let ti = |i: usize| ident!("t" i);
    #[allow(non_snake_case)]
    let Ti = |i: usize| ident!("T" i);
    #[allow(non_snake_case)]
    let Mi = |i: usize| ident!("M" i);
    let mutator_i = |i: usize| ident!("mutator_" i);
    let get_i = |i: usize| ts!(ident!("get_" i) "()" );
    let get_i_mut = |i: usize| ts!(ident!("get_" i "_mut") "()");
    let ti_value = |i: usize| ident!("t" i "_value");
    let ti_cache = |i: usize| ident!("t" i "_cache");
    // let get_ = |i: usize| ident!("get_" i);

    extend_ts!(tb,"
    impl <T , " type_params " > " mutator "<T> for " name_mutator "< " type_params " >
    where
        T: " clone "," 
        join_ts!(0..nbr_elements, i,
            Ti(i) ":" clone ","
            Mi(i) ":" mutator "<" Ti(i) ">,"
        ) "
        T: " tuple_structure "<" join_ts!(0..nbr_elements, i, Ti(i) "=" Ti(i), separator: ",") ">,
    {
        type Cache = Cache <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" mutator "<" Ti(i) "> >::Cache "
            , separator: ",")
        ">;
        type MutationStep = MutationStep <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" mutator "<" Ti(i) "> >::MutationStep "
            , separator: ",")
        ">;
        type ArbitraryStep = ArbitraryStep <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" mutator "<" Ti(i) "> >::ArbitraryStep "
            , separator: ",")
        ">;
        type UnmutateToken = UnmutateToken <"
            join_ts!(0..nbr_elements, i,
                "<" Mi(i) "as" mutator "<" Ti(i) "> >::UnmutateToken "
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
        fn complexity(&self, _value: &T, cache: &Self::Cache) -> f64 {
            cache.cplx
        }
        fn cache_from_value(&self, value: &T) -> Self::Cache {"
            join_ts!(0..nbr_elements, i,
                "let" ti(i) "= self." mutator_i(i) ".cache_from_value(value." get_i(i) ");"
            )
            "let cplx = "
                join_ts!(0..nbr_elements, i,
                    "self." mutator_i(i) ".complexity(value." get_i(i) ", &" ti(i) ")"
                , separator: "+") ";"
            "Self::Cache {"
                join_ts!(0..nbr_elements, i, ti(i) ",")
                "cplx
            }
        }
        fn initial_step_from_value(&self, value: &T) -> Self::MutationStep {"
            join_ts!(0..nbr_elements, i,
                "let" ti(i) "= self." mutator_i(i) ".initial_step_from_value(value." get_i(i) ");"
            )
            "let step = 0;"
            "Self::MutationStep {"
                join_ts!(0..nbr_elements, i, ti(i) ",")
                "inner: vec![" join_ts!(0..nbr_elements, i, "InnerMutationStep::" Ti(i) ",") "] ,
                step
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
                    "_ => unreachable!()
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

        fn ordered_mutate(
            &mut self,
            value: &mut T,
            cache: &mut Self::Cache,
            step: &mut Self::MutationStep,
            max_cplx: f64,
        ) -> " option "<Self::UnmutateToken> {
            if step.inner.is_empty() {
                return " none ";
            }
            let orig_step = step.step;
            step.step += 1;
            let current_cplx = self.complexity(value, cache);
            let inner_step_to_remove: usize;

            match step.inner[orig_step % step.inner.len()] {"
            join_ts!(0..nbr_elements, i,
                "InnerMutationStep::" Ti(i) "=> {
                    let current_field_cplx = self." mutator_i(i) ".complexity(value." get_i(i) ", &cache." ti(i) ");
                    let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                    if let " some "(token) =
                        self." mutator_i(i) "
                            .ordered_mutate(value." get_i_mut(i) ", &mut cache." ti(i) ", &mut step." ti(i) ", max_field_cplx)
                    {
                        let new_field_complexity = self." mutator_i(i) ".complexity(value." get_i(i) ", &cache." ti(i) ");
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
            self.ordered_mutate(value, cache, step, max_cplx)
        }
        "
        // TODO!
        "
        fn random_mutate(&mut self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
            let current_cplx = self.complexity(value, cache);
            match self.rng.usize(..) % " nbr_elements " {"
                join_ts!(0..nbr_elements, i,
                    i "=> {
                        let current_field_cplx = self." mutator_i(i) ".complexity(value." get_i(i) ", &cache." ti(i) ");
                        let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                        let token = self." mutator_i(i) "
                            .random_mutate(value." get_i_mut(i) ", &mut cache." ti(i) ", max_field_cplx) ;
                    
                        let new_field_complexity = self." mutator_i(i) ".complexity(value." get_i(i) ", &cache." ti(i) ");
                        cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                        return Self::UnmutateToken {
                            " ti(i) ": " some "(token),
                            cplx: current_cplx,
                            ..Self::UnmutateToken::default()
                        };
                    }"
                )
                "_ => unreachable!()
            }
        }
        fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
            cache.cplx = t.cplx;"
            join_ts!(0..nbr_elements, i,
                "if let" some "(subtoken) = t." ti(i) "{
                    self. " mutator_i(i) ".unmutate(value." get_i_mut(i) ", &mut cache. " ti(i) " , subtoken);
                }"
            )
        "}
    }
    "
    )
}

#[cfg(test)]
mod test {
    use decent_synquote_alternative::token_builder;
    use proc_macro2::TokenStream;
    use token_builder::TokenBuilder;

    use super::{
        declare_tuple_mutator, declare_tuple_mutator_helper_types, impl_mutator_trait, make_tuple_type_structure,
    };

    #[test]
    fn test_impl_mutator_trait() {
        let mut tb = TokenBuilder::new();
        impl_mutator_trait(&mut tb, 2);
        let generated = tb.end().to_string();

        let expected = "
impl<T, T0, T1, M0, M1> fuzzcheck_traits::Mutator<T> for Tuple2Mutator<T0, T1, M0, M1>
where
    T: ::std::clone::Clone,
    T0: ::std::clone::Clone,
    M0: fuzzcheck_traits::Mutator<T0>,
    T1: ::std::clone::Clone,
    M1: fuzzcheck_traits::Mutator<T1>,
    T: Tuple2Structure<T0 = T0, T1 = T1>,
{
    type Cache = Cache< <M0 as fuzzcheck_traits::Mutator<T0> >::Cache, <M1 as fuzzcheck_traits::Mutator<T1> >::Cache>;
    type MutationStep = MutationStep<
        <M0 as fuzzcheck_traits::Mutator<T0> >::MutationStep,
        <M1 as fuzzcheck_traits::Mutator<T1> >::MutationStep
    >;
    type ArbitraryStep = ArbitraryStep<
        <M0 as fuzzcheck_traits::Mutator<T0> >::ArbitraryStep,
        <M1 as fuzzcheck_traits::Mutator<T1> >::ArbitraryStep
    >;
    type UnmutateToken = UnmutateToken<
        <M0 as fuzzcheck_traits::Mutator<T0> >::UnmutateToken,
        <M1 as fuzzcheck_traits::Mutator<T1> >::UnmutateToken
    >;

    fn max_complexity(&self) -> f64 {
        self.mutator_0.max_complexity() + self.mutator_1.max_complexity()
    }
    fn min_complexity(&self) -> f64 {
        self.mutator_0.min_complexity() + self.mutator_1.min_complexity()
    }
    fn complexity(&self, _value: &T, cache: &Self::Cache) -> f64 {
        cache.cplx
    }
    fn cache_from_value(&self, value: &T) -> Self::Cache {
        let t0 = self.mutator_0.cache_from_value(value.get_0());
        let t1 = self.mutator_1.cache_from_value(value.get_1());
        let cplx = self.mutator_0.complexity(value.get_0(), &t0) + self.mutator_1.complexity(value.get_1(), &t1);
        Self::Cache { t0, t1, cplx }
    }
    fn initial_step_from_value(&self, value: &T) -> Self::MutationStep {
        let t0 = self.mutator_0.initial_step_from_value(value.get_0());
        let t1 = self.mutator_1.initial_step_from_value(value.get_1());
        let step = 0;
        Self::MutationStep {
            t0,
            t1,
            inner: vec![InnerMutationStep::T0, InnerMutationStep::T1,],
            step
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
                _ => unreachable!()
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

    fn ordered_mutate(
        &mut self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> ::std::option::Option<Self::UnmutateToken> {
        if step.inner.is_empty() {
            return ::std::option::Option::None;
        }
        let orig_step = step.step;
        step.step += 1;
        let current_cplx = self.complexity(value, cache);
        let inner_step_to_remove: usize;

        match step.inner[orig_step % step.inner.len()] {
            InnerMutationStep::T0 => {
                let current_field_cplx = self.mutator_0.complexity(value.get_0(), &cache.t0);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                if let ::std::option::Option::Some(token) =
                    self.mutator_0
                        .ordered_mutate(value.get_0_mut(), &mut cache.t0, &mut step.t0, max_field_cplx)
                {
                    let new_field_complexity = self.mutator_0.complexity(value.get_0(), &cache.t0);
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
                let current_field_cplx = self.mutator_1.complexity(value.get_1(), &cache.t1);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                if let ::std::option::Option::Some(token) =
                    self.mutator_1
                        .ordered_mutate(value.get_1_mut(), &mut cache.t1, &mut step.t1, max_field_cplx)
                {
                    let new_field_complexity = self.mutator_1.complexity(value.get_1(), &cache.t1);
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
        self.ordered_mutate(value, cache, step, max_cplx)
    }
    fn random_mutate(&mut self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        let current_cplx = self.complexity(value, cache);
        match self.rng.usize(..) % 2 {
            0 => {
                let current_field_cplx = self.mutator_0.complexity(value.get_0(), &cache.t0);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                let token = self
                    .mutator_0
                    .random_mutate(value.get_0_mut(), &mut cache.t0, max_field_cplx);
                let new_field_complexity = self.mutator_0.complexity(value.get_0(), &cache.t0);
                cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                return Self::UnmutateToken {
                    t0: ::std::option::Option::Some(token),
                    cplx: current_cplx,
                    ..Self::UnmutateToken::default()
                };
            }
            1 => {
                let current_field_cplx = self.mutator_1.complexity(value.get_1(), &cache.t1);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                let token = self
                    .mutator_1
                    .random_mutate(value.get_1_mut(), &mut cache.t1, max_field_cplx);
                let new_field_complexity = self.mutator_1.complexity(value.get_1(), &cache.t1);
                cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                return Self::UnmutateToken {
                    t1: ::std::option::Option::Some(token),
                    cplx: current_cplx,
                    ..Self::UnmutateToken::default()
                };
            }
            _ => unreachable!()
        }
    }
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        cache.cplx = t.cplx;
        if let ::std::option::Option::Some(subtoken) = t.t0 {
            self.mutator_0.unmutate(value.get_0_mut(), &mut cache.t0, subtoken);
        }
        if let ::std::option::Option::Some(subtoken) = t.t1 {
            self.mutator_1.unmutate(value.get_1_mut(), &mut cache.t1, subtoken);
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
    M0: fuzzcheck_traits::Mutator<T0> ,
    T1: ::std::clone::Clone,
    M1: fuzzcheck_traits::Mutator<T1>
{
    pub mutator_0: M0,
    pub mutator_1: M1,
    rng: fuzzcheck_mutators ::fastrand::Rng,
    _phantom: ::std::marker::PhantomData<(T0, T1)>
}
impl<T0, T1, M0, M1> Tuple2Mutator<T0, T1, M0, M1>
where
    T0: ::std::clone::Clone,
    M0: fuzzcheck_traits::Mutator<T0> ,
    T1: ::std::clone::Clone,
    M1: fuzzcheck_traits::Mutator<T1>
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
        M: fuzzcheck_traits::Mutator<T0>
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
        M: fuzzcheck_traits::Mutator<T1>
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
        pub trait Tuple2Structure {
            type T0;
            fn get_0(&self) -> &Self::T0;
            fn get_0_mut(&mut self) -> &mut Self::T0;
        
            type T1;
            fn get_1(&self) -> &Self::T1;
            fn get_1_mut(&mut self) -> &mut Self::T1;
        
            fn new(t: (Self::T0, Self::T1)) -> Self;
        }

        impl<T0, T1> Tuple2Structure for (T0, T1) {
            type T0 = T0;
            fn get_0(&self) -> & Self :: T0 {
                &self.0
            }

            fn get_0_mut(&mut self) -> &mut Self :: T0 {
                &mut self.0
            }
            type T1 = T1;

            fn get_1(&self) -> & Self :: T1 {
                &self.1
            }

            fn get_1_mut(&mut self) -> &mut Self :: T1 {
                &mut self.1
            }
            fn new(t: (Self::T0, Self::T1)) -> Self {
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
