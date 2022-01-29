/*!
Types implementing the [Mutator] trait.

This module provides the following mutators:

* mutators for basic types such as
    * `bool` ([here](crate::mutators::bool::BoolMutator))
    * `char` ([here](crate::mutators::char::CharWithinRangeMutator) and [here](crate::mutators::character_classes::CharacterMutator))
    * integers ([here](crate::mutators::integer) and [here](crate::mutators::integer_within_range))
    * `Vec` ([here](crate::mutators::vector::VecMutator) and [here](crate::mutators::fixed_len_vector::FixedLenVecMutator))
    * `Option` ([here](crate::mutators::option::OptionMutator))
    * `Result` ([here](crate::mutators::result::ResultMutator))
    * `Box` ([here](crate::mutators::boxed))
    * tuples of up to 10 elements ([here](crate::mutators::tuples))

* procedural macros to generate mutators for custom types:
    * [`#[derive(DefaultMutator)]`](fuzzcheck_mutators_derive::DefaultMutator) which works on most structs and enums
    * [`make_mutator! { .. }`](fuzzcheck_mutators_derive::make_mutator) which works like `#[derive(DefaultMutator)]` but is customisable
*/
#![cfg_attr(
    feature = "grammar_mutator",
    doc = "* grammar-based string and syntax tree mutators ([here](crate::mutators::grammar)) __(supported on crate feature `grammar_mutator` only)__"
)]
#![cfg_attr(
    not(feature = "grammar_mutator"),
    doc = "* ~~grammar-based string and syntax tree mutators~~ (note: you are viewing the documentation of fuzzcheck without the `grammar_mutator` feature. Therefore, grammar-based mutators are not available)"
)]
/*!
- basic blocks to build more complex mutators:
    * [`DictionaryMutator<_, M>`](crate::mutators::dictionary::DictionaryMutator) to wrap a mutator and prioritise the generation of a few given values
    * [`AlternationMutator<_, M>`](crate::mutators::alternation::AlternationMutator) to use multiple different mutators acting on the same test case type
    * [`Either<M1, M2>`](crate::mutators::either::Either) is the regular `Either` type, which also implements `Mutator<T>` if both `M1` and `M2` implement it too
    * [`RecursiveMutator` and `RecurToMutator`](crate::mutators::recursive) are wrappers allowing mutators to call themselves recursively, which is necessary to mutate recursive types.
    * [`MapMutator<..>`](crate::mutators::map::MapMutator) wraps a mutator and transforms the generated value using a user-provided function.
*/
pub mod alternation;
// pub mod arc;
// pub mod array;
pub mod bool;
// pub mod boxed;
// pub mod char;
// pub mod character_classes;
// pub mod dictionary;
// pub mod either;
pub mod enums;
// pub mod filter;
// pub mod fixed_len_vector;
// #[cfg(feature = "grammar_mutator")]
// #[doc(cfg(feature = "grammar_mutator"))]
// pub mod grammar;
pub mod integer;
// pub mod integer_within_range;
// pub mod map;
pub mod mutations;
// pub mod never;
pub mod option;
// pub mod range;
// pub mod rc;
pub mod recursive;
// pub mod result;
// pub mod string;
pub mod tuples;
pub mod unit;
pub mod vector;
pub mod vose_alias;
pub mod wrapper;
use crate::Mutator;
use std::ops::Range;

/// A trait for giving a type a default [Mutator]
pub trait DefaultMutator: Clone {
    type Mutator: Mutator<Self>;
    fn default_mutator() -> Self::Mutator;
}

/// Generate a random f64 within the given range
/// The start and end of the range must be finite
/// This is a very naive implementation
#[no_coverage]
#[inline]
pub(crate) fn gen_f64(rng: &fastrand::Rng, range: Range<f64>) -> f64 {
    range.start + rng.f64() * (range.end - range.start)
}

#[must_use]
#[no_coverage]
fn size_to_cplxity(size: usize) -> f64 {
    (usize::BITS - (size.saturating_sub(1)).leading_zeros()) as f64
}

#[cfg(test)]
mod test {
    use crate::mutators::size_to_cplxity;

    #[allow(clippy::float_cmp)]
    #[test]
    #[no_coverage]
    fn test_size_to_cplxity() {
        assert_eq!(0.0, size_to_cplxity(0));
        assert_eq!(0.0, size_to_cplxity(1));
        assert_eq!(1.0, size_to_cplxity(2));
        assert_eq!(2.0, size_to_cplxity(3));
        assert_eq!(2.0, size_to_cplxity(4));
        assert_eq!(3.0, size_to_cplxity(5));
        assert_eq!(3.0, size_to_cplxity(8));
        assert_eq!(5.0, size_to_cplxity(31));
    }
}

#[doc(hidden)]
pub mod testing_utilities {
    use std::collections::HashSet;
    use std::fmt::Debug;
    use std::hash::Hash;

    use crate::Mutator;

    #[no_coverage]
    pub fn test_mutator<T, M>(
        m: M,
        maximum_complexity_arbitrary: f64,
        maximum_complexity_mutate: f64,
        avoid_duplicates: bool,
        check_consistent_complexities: bool,
        nbr_arbitraries: usize,
        nbr_mutations: usize,
    ) where
        M: Mutator<T>,
        T: Clone + Debug + PartialEq + Eq + Hash,
        M::Cache: Clone,
    {
        let mut arbitrary_step = m.default_arbitrary_step();

        let mut arbitraries = HashSet::new();
        for _i in 0..nbr_arbitraries {
            if let Some((x, cplx)) = m.ordered_arbitrary(&mut arbitrary_step, maximum_complexity_arbitrary) {
                // assert!(
                //     cplx <= maximum_complexity_arbitrary,
                //     "{} {}",
                //     cplx,
                //     maximum_complexity_arbitrary
                // );
                if avoid_duplicates {
                    let is_new = arbitraries.insert(x.clone());
                    assert!(is_new);
                }
                let cache = m.validate_value(&x).unwrap();
                let mut mutation_step = m.default_mutation_step(&x, &cache);
                let other_cplx = m.complexity(&x, &cache);
                if check_consistent_complexities {
                    assert!((cplx - other_cplx).abs() < 0.01, "{:.3} != {:.3}", cplx, other_cplx);
                }

                let mut mutated = HashSet::new();
                if avoid_duplicates {
                    mutated.insert(x.clone());
                }
                let mut x_mut = x.clone();
                let mut cache_mut = cache.clone();
                for _j in 0..nbr_mutations {
                    if let Some((token, cplx)) = m.ordered_mutate(
                        &mut x_mut,
                        &mut cache_mut,
                        &mut mutation_step,
                        maximum_complexity_mutate,
                    ) {
                        // assert!(
                        //     cplx <= maximum_complexity_mutate,
                        //     "{} {}",
                        //     cplx,
                        //     maximum_complexity_mutate
                        // );
                        if avoid_duplicates {
                            let is_new = mutated.insert(x_mut.clone());
                            assert!(is_new);
                        }

                        let validated = m.validate_value(&x_mut).unwrap();
                        let other_cplx = m.complexity(&x_mut, &validated);
                        if check_consistent_complexities {
                            assert!(
                                (cplx - other_cplx).abs() < 0.01,
                                "{:.3} != {:.3} for {:?}",
                                cplx,
                                other_cplx,
                                x_mut
                            );
                        }
                        m.unmutate(&mut x_mut, &mut cache_mut, token);
                        assert_eq!(x, x_mut);
                        // assert_eq!(cache, cache_mut);
                    } else {
                        // println!("Stopped mutating at {}", j);
                        break;
                    }
                }
            } else {
                // println!("Stopped arbitraries at {}", i);
                break;
            }
        }
        for _i in 0..nbr_arbitraries {
            let (x, _cplx) = m.random_arbitrary(maximum_complexity_arbitrary);
            let cache = m.validate_value(&x).unwrap();
            // let mutation_step = m.default_mutation_step(&x, &cache);
            let _other_cplx = m.complexity(&x, &cache);
            // assert!((cplx - other_cplx).abs() < 0.01, "{:.3} != {:.3}", cplx, other_cplx);
            let mut x_mut = x.clone();
            let mut cache_mut = cache.clone();
            for _j in 0..nbr_mutations {
                let (token, cplx) = m.random_mutate(&mut x_mut, &mut cache_mut, maximum_complexity_mutate);
                let validated = m.validate_value(&x_mut).unwrap();
                let other_cplx = m.complexity(&x_mut, &validated);
                if check_consistent_complexities {
                    assert!((cplx - other_cplx).abs() < 0.01, "{:.3} != {:.3}", cplx, other_cplx);
                }
                m.unmutate(&mut x_mut, &mut cache_mut, token);
                assert_eq!(x, x_mut);
                // assert_eq!(cache, cache_mut);
            }
        }
    }
    #[no_coverage]
    pub fn bench_mutator<T, M>(
        m: M,
        maximum_complexity_arbitrary: f64,
        maximum_complexity_mutate: f64,
        nbr_arbitraries: usize,
        nbr_mutations: usize,
    ) where
        M: Mutator<T>,
        T: Clone + Debug + PartialEq + Eq + Hash,
        M::Cache: Clone,
    {
        let mut arbitrary_step = m.default_arbitrary_step();
        for _i in 0..nbr_arbitraries {
            if let Some((mut x, _cplx)) = m.ordered_arbitrary(&mut arbitrary_step, maximum_complexity_arbitrary) {
                let mut cache = m.validate_value(&x).unwrap();
                let mut mutation_step = m.default_mutation_step(&x, &cache);
                let _other_cplx = m.complexity(&x, &cache);
                for _j in 0..nbr_mutations {
                    if let Some((token, _cplx)) =
                        m.ordered_mutate(&mut x, &mut cache, &mut mutation_step, maximum_complexity_mutate)
                    {
                        m.unmutate(&mut x, &mut cache, token);
                    } else {
                        break;
                    }
                }
            } else {
                break;
            }
        }
        for _i in 0..nbr_arbitraries {
            let (mut x, _cplx) = m.random_arbitrary(maximum_complexity_arbitrary);
            let mut cache = m.validate_value(&x).unwrap();
            let _other_cplx = m.complexity(&x, &cache);
            for _j in 0..nbr_mutations {
                let (token, _cplx) = m.random_mutate(&mut x, &mut cache, maximum_complexity_mutate);
                m.unmutate(&mut x, &mut cache, token);
            }
        }
    }
}
