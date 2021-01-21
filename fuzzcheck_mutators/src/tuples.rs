fuzzcheck_mutators_derive::make_basic_tuple_mutator!(2 crate);
fuzzcheck_mutators_derive::make_basic_tuple_mutator!(3 crate);
fuzzcheck_mutators_derive::make_basic_tuple_mutator!(4 crate);
fuzzcheck_mutators_derive::make_basic_tuple_mutator!(5 crate);
fuzzcheck_mutators_derive::make_basic_tuple_mutator!(6 crate);
fuzzcheck_mutators_derive::make_basic_tuple_mutator!(7 crate);
fuzzcheck_mutators_derive::make_basic_tuple_mutator!(8 crate);
fuzzcheck_mutators_derive::make_basic_tuple_mutator!(9 crate);
fuzzcheck_mutators_derive::make_basic_tuple_mutator!(10 crate);

// use fuzzcheck_traits::Mutator;

// impl<T0, T1, M0, M1> Default for Tuple2Mutator<T0, T1, M0, M1>
// where
//     T0: Clone,
//     T1: Clone,
//     M0: Mutator<T0>,
//     M1: Mutator<T1>,
//     M0: Default,
//     M1: Default,
// {
//     fn default() -> Self {
//         Self::new(M0::default(), M1::default())
//     }
// }

/*
 impl< T, " type_params ">" Default "for" name_mutator "< T, " type_params ">
        where
        T: " clone ","
        join_ts!(0..nbr_elements, i,
            Ti(i) ":" clone ","
            Mi(i) ":" mutator "<" Ti(i) ">,"
        ) "
        T: " tuple_structure "<" join_ts!(0..nbr_elements, i, Ti(i) "=" Ti(i), separator: ",") ">,"
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
*/
