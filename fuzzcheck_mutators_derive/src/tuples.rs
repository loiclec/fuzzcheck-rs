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
                fn" ident!("get_" i) "( & self ) -> & Self ::" ident!("T" i) " ;
                fn" ident!("get_" i "_mut") "( & mut self ) -> & mut Self ::" ident!("T" i) " ;
                "
            )
            "fn new ( t : ( " join_ts!(0 .. nbr_elements, i, "Self ::" ident!("T" i), separator: "," )" ) ) -> Self ;"
        "}
        
        impl < " sequence_of_type_params ">" name_trait " for " tuple_type " {"
            join_ts!(0..nbr_elements, i,
                "type" ident!("T" i)" = " ident!("T" i) " ;
                fn" ident!("get_" i) "( & self ) -> & Self ::" ident!("T" i) " {
                    & self . " i "
                }
                fn" ident!("get_" i "_mut") "( & mut self ) -> & mut Self ::" ident!("T" i) " {
                    & mut self . " i "
                }
                "
            )
            "fn new ( t : ( " join_ts!(0 .. nbr_elements, i, "Self ::" ident!("T" i), separator: "," )" ) ) -> Self {
                t
            }"
        "}"
    );
}

fn declare_tuple_mutator(tb: &mut TokenBuilder, nbr_elements: usize) {
    let clone = ts!(":: std :: clone :: Clone");
    let mutator = ts!("fuzzcheck_traits :: Mutator");
    let rng = ts!("fuzzcheck_mutators :: fastrand :: Rng");
    let phantom_data = ts!(":: std :: marker :: PhantomData");

    let name_mutator = ident!("Tuple" nbr_elements "Mutator");

    let tuple_type_params = join_ts!(0..nbr_elements, i, ident!("T" i), separator: ",");
    let mutator_type_params = join_ts!(0..nbr_elements, i, ident!("M" i), separator: ",");
    let type_params = ts!(tuple_type_params "," mutator_type_params);
    let tuple_type = ts!("(" tuple_type_params ")");

    let where_clause = ts!(
        "where"
        join_ts!(0..nbr_elements, i,
            ident!("T" i) ":" clone ","
            ident!("M" i) ":" mutator " < " ident!("T" i) " >"
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
            pub fn new ( " join_ts!(0..nbr_elements, i, ident!("mutator_" i) ":" ident!("M" i), separator: ",") " ) -> Self {
                Self {"
                    join_ts!(0..nbr_elements, i,
                        ident!("mutator_" i) ","
                    )
                    "rng : < _ > :: default ( ) ,
                    _phantom :" phantom_data
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
                        "rng : self . rng ,
                        _phantom : self . _phantom
                    }
                }"
            )
        "}"
    )
}

#[cfg(test)]
mod test {
    use decent_synquote_alternative::token_builder;
    use proc_macro2::TokenStream;
    use token_builder::TokenBuilder;

    use super::{declare_tuple_mutator, make_tuple_type_structure};

    #[test]
    fn test_declare_tuple_mutator() {
        let mut tb = TokenBuilder::new();
        declare_tuple_mutator(&mut tb, 2);
        let generated = tb.end().to_string();

        let expected = r#"  
pub struct Tuple2Mutator<T0, T1, M0, M1>
where
    T0: :: std :: clone :: Clone,
    M0: fuzzcheck_traits :: Mutator<T0> ,
    T1: :: std :: clone :: Clone,
    M1: fuzzcheck_traits :: Mutator<T1>
{
    pub mutator_0: M0,
    pub mutator_1: M1,
    rng: fuzzcheck_mutators :: fastrand :: Rng,
    _phantom: :: std :: marker :: PhantomData<(T0, T1)>
}
impl<T0, T1, M0, M1> Tuple2Mutator<T0, T1, M0, M1>
where
    T0: :: std :: clone :: Clone,
    M0: fuzzcheck_traits :: Mutator<T0> ,
    T1: :: std :: clone :: Clone,
    M1: fuzzcheck_traits :: Mutator<T1>
{
    pub fn new(mutator_0: M0, mutator_1: M1) -> Self {
        Self {
            mutator_0,
            mutator_1,
            rng: < _ > :: default() ,
            _phantom: :: std :: marker :: PhantomData
        }
    }

    pub fn replacing_mutator_0<M>(self, mutator: M) -> Tuple2Mutator<T0, T1, M, M1>
    where
        M: fuzzcheck_traits :: Mutator<T0>
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
        M: fuzzcheck_traits :: Mutator<T1>
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
