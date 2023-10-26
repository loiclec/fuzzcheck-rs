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
    * [`AlternationMutator<_, M>`](crate::mutators::alternation::AlternationMutator) to use multiple different mutators acting on the same test case type
    * [`Either<M1, M2>`](crate::mutators::either::Either) is the regular `Either` type, which also implements `Mutator<T>` if both `M1` and `M2` implement it too
    * [`RecursiveMutator` and `RecurToMutator`](crate::mutators::recursive) are wrappers allowing mutators to call themselves recursively, which is necessary to mutate recursive types.
    * [`MapMutator<..>`](crate::mutators::map::MapMutator) wraps a mutator and transforms the generated value using a user-provided function.
*/

pub const CROSSOVER_RATE: u8 = 10;

use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::ops::Range;

use ahash::AHashMap;

use self::filter::FilterMutator;
use self::map::MapMutator;
use crate::subvalue_provider::Generation;
use crate::{Mutator, SubValueProvider};

pub mod alternation;
pub mod arc;
pub mod array;
pub mod bool;
pub mod boxed;
pub mod char;
pub mod character_classes;
pub mod cow;
pub mod either;
pub mod enums;
pub mod filter;
pub mod fixed_len_vector;
#[cfg(feature = "grammar_mutator")]
#[doc(cfg(feature = "grammar_mutator"))]
pub mod grammar;
pub mod integer;
pub mod integer_within_range;
pub mod map;
pub mod mutations;
pub mod never;
pub mod option;
pub mod range;
pub mod rc;
pub mod recursive;
pub mod result;
pub mod string;
pub mod tuples;
pub mod unique;
pub mod unit;
pub mod vector;
pub mod vose_alias;

/// A trait for giving a type a default [Mutator]
pub trait DefaultMutator: Clone + 'static {
    type Mutator: Mutator<Self>;
    fn default_mutator() -> Self::Mutator;
}

#[derive(Clone)]
pub struct CrossoverStep<T> {
    steps: AHashMap<usize, (Generation, usize)>,
    _phantom: PhantomData<T>,
}
impl<T> Default for CrossoverStep<T> {
    #[coverage(off)]
    fn default() -> Self {
        CrossoverStep {
            steps: <_>::default(),
            _phantom: <_>::default(),
        }
    }
}

impl<T> CrossoverStep<T>
where
    T: 'static,
{
    #[coverage(off)]
    pub fn get_next_subvalue<'a>(
        &mut self,
        subvalue_provider: &'a dyn SubValueProvider,
        max_cplx: f64,
    ) -> Option<(&'a T, f64)> {
        // TODO: mark an entry as exhausted?
        let id = subvalue_provider.identifier();
        let entry = self.steps.entry(id.idx).or_insert((id.generation, 0));
        if entry.0 < id.generation {
            entry.0 = id.generation;
            entry.1 = 0;
        }
        subvalue_provider
            .get_subvalue(TypeId::of::<T>(), max_cplx, &mut entry.1)
            .map(
                #[coverage(off)]
                |(x, cplx)| (x.downcast_ref::<T>().unwrap(), cplx),
            )
    }
}

#[coverage(off)]
fn keep_orig_cplx<T>(_x: &T, cplx: f64) -> f64 {
    cplx
}

/// A trait for convenience methods automatically implemented for all types that conform to `Mutator<V>`
pub trait MutatorExt<T>: Mutator<T> + Sized
where
    T: Clone + 'static,
{
    /// Create a mutator which wraps `self` but only produces values
    /// for which the given closure returns `true`
    #[coverage(off)]
    fn filter<F>(self, filter: F) -> FilterMutator<Self, F>
    where
        F: Fn(&T) -> bool + 'static,
    {
        FilterMutator::new(self, filter)
    }
    /// Create a mutator which wraps `self` and transforms the values generated by `self`
    /// using the `map` closure. The second closure, `parse`, should apply the opposite
    /// transformation.
    #[coverage(off)]
    fn map<To, Map, Parse>(self, map: Map, parse: Parse) -> MapMutator<T, To, Self, Parse, Map, fn(&To, f64) -> f64>
    where
        To: Clone + 'static,
        Map: Fn(&T) -> To,
        Parse: Fn(&To) -> Option<T>,
    {
        MapMutator::new(self, parse, map, keep_orig_cplx)
    }
}
impl<T, M> MutatorExt<T> for M
where
    M: Mutator<T>,
    T: Clone + 'static,
{
}

/**
 A trait for types that are basic wrappers over a mutator, such as `Box<M>`.

 Such wrapper types automatically implement the [`Mutator`](Mutator) trait.
*/
pub trait MutatorWrapper {
    type Wrapped;

    fn wrapped_mutator(&self) -> &Self::Wrapped;
}

impl<T: Clone + 'static, W, M> Mutator<T> for M
where
    M: MutatorWrapper<Wrapped = W>,
    W: Mutator<T>,
    Self: 'static,
{
    #[doc(hidden)]
    type Cache = W::Cache;
    #[doc(hidden)]
    type MutationStep = W::MutationStep;
    #[doc(hidden)]
    type ArbitraryStep = W::ArbitraryStep;
    #[doc(hidden)]
    type UnmutateToken = W::UnmutateToken;

    #[doc(hidden)]
    #[coverage(off)]
    fn initialize(&self) {
        self.wrapped_mutator().initialize()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.wrapped_mutator().default_arbitrary_step()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn is_valid(&self, value: &T) -> bool {
        self.wrapped_mutator().is_valid(value)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        self.wrapped_mutator().validate_value(value)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        self.wrapped_mutator().default_mutation_step(value, cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        self.wrapped_mutator().global_search_space_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        self.wrapped_mutator().max_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        self.wrapped_mutator().min_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.wrapped_mutator().complexity(value, cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        self.wrapped_mutator().ordered_arbitrary(step, max_cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        self.wrapped_mutator().random_arbitrary(max_cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        self.wrapped_mutator()
            .ordered_mutate(value, cache, step, subvalue_provider, max_cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        self.wrapped_mutator().random_mutate(value, cache, max_cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.wrapped_mutator().unmutate(value, cache, t)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, value: &'a T, cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        self.wrapped_mutator().visit_subvalues(value, cache, visit)
    }
}

impl<M> MutatorWrapper for Box<M> {
    type Wrapped = M;
    #[coverage(off)]
    fn wrapped_mutator(&self) -> &Self::Wrapped {
        self.as_ref()
    }
}

pub struct Wrapper<T>(pub T);
impl<T> MutatorWrapper for Wrapper<T> {
    type Wrapped = T;
    #[coverage(off)]
    fn wrapped_mutator(&self) -> &Self::Wrapped {
        &self.0
    }
}

/// Generate a random f64 within the given range
/// The start and end of the range must be finite
/// This is a very naive implementation
#[coverage(off)]
#[inline]
pub(crate) fn gen_f64(rng: &fastrand::Rng, range: Range<f64>) -> f64 {
    range.start + rng.f64() * (range.end - range.start)
}

#[must_use]
#[coverage(off)]
fn size_to_cplxity(size: usize) -> f64 {
    (usize::BITS - size.leading_zeros()) as f64
}

#[cfg(test)]
mod test {
    use crate::mutators::size_to_cplxity;

    #[allow(clippy::float_cmp)]
    #[test]
    #[coverage(off)]
    fn test_size_to_cplxity() {
        assert_eq!(0.0, size_to_cplxity(0));
        assert_eq!(1.0, size_to_cplxity(1));
        assert_eq!(2.0, size_to_cplxity(2));
        assert_eq!(2.0, size_to_cplxity(3));
        assert_eq!(3.0, size_to_cplxity(4));
        assert_eq!(3.0, size_to_cplxity(5));
        assert_eq!(4.0, size_to_cplxity(8));
        assert_eq!(5.0, size_to_cplxity(31));
    }
}

#[doc(hidden)]
pub mod testing_utilities {
    use std::collections::HashSet;
    use std::fmt::Debug;
    use std::hash::Hash;

    use crate::subvalue_provider::EmptySubValueProvider;
    use crate::Mutator;

    #[coverage(off)]
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
        T: Clone + Debug + PartialEq + Eq + Hash + 'static,
        M::Cache: Clone,
    {
        m.initialize();
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
                        &EmptySubValueProvider,
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
                                "{:.3} != {:.3} for {:?} mutated from {:?}",
                                cplx,
                                other_cplx,
                                x_mut,
                                x
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
        // for _i in 0..nbr_arbitraries {
        //     let (x, cplx) = m.random_arbitrary(maximum_complexity_arbitrary);
        //     let cache = m.validate_value(&x).unwrap();
        //     // let mutation_step = m.default_mutation_step(&x, &cache);
        //     let other_cplx = m.complexity(&x, &cache);
        //     assert!((cplx - other_cplx).abs() < 0.01, "{:.3} != {:.3}", cplx, other_cplx);
        //     let mut x_mut = x.clone();
        //     let mut cache_mut = cache.clone();
        //     for _j in 0..nbr_mutations {
        //         let (token, cplx) = m.random_mutate(&mut x_mut, &mut cache_mut, maximum_complexity_mutate);
        //         let validated = m.validate_value(&x_mut).unwrap();
        //         let other_cplx = m.complexity(&x_mut, &validated);
        //         if check_consistent_complexities {
        //             assert!((cplx - other_cplx).abs() < 0.01, "{:.3} != {:.3}", cplx, other_cplx);
        //         }
        //         m.unmutate(&mut x_mut, &mut cache_mut, token);
        //         assert_eq!(x, x_mut);
        //         // assert_eq!(cache, cache_mut);
        //     }
        // }
    }
    // #[coverage(off)]
    // pub fn bench_mutator<T, M>(
    //     m: M,
    //     maximum_complexity_arbitrary: f64,
    //     maximum_complexity_mutate: f64,
    //     nbr_arbitraries: usize,
    //     nbr_mutations: usize,
    // ) where
    //     M: Mutator<T>,
    //     T: Clone + Debug + PartialEq + Eq + Hash + 'static,
    //     M::Cache: Clone,
    // {
    //     let mut arbitrary_step = m.default_arbitrary_step();
    //     for _i in 0..nbr_arbitraries {
    //         if let Some((mut x, cplx)) = m.ordered_arbitrary(&mut arbitrary_step, maximum_complexity_arbitrary) {
    //             let mut cache = m.validate_value(&x).unwrap();
    //             let mut mutation_step = m.default_mutation_step(&x, &cache);
    //             let other_cplx = m.complexity(&x, &cache);
    //             for _j in 0..nbr_mutations {
    //                 if let Some((token, _cplx)) =
    //                     m.ordered_mutate(&mut x, &mut cache, &mut mutation_step, maximum_complexity_mutate)
    //                 {
    //                     m.unmutate(&mut x, &mut cache, token);
    //                 } else {
    //                     break;
    //                 }
    //             }
    //         } else {
    //             break;
    //         }
    //     }
    //     for _i in 0..nbr_arbitraries {
    //         let (mut x, cplx) = m.random_arbitrary(maximum_complexity_arbitrary);
    //         let mut cache = m.validate_value(&x).unwrap();
    //         let other_cplx = m.complexity(&x, &cache);
    //         for _j in 0..nbr_mutations {
    //             let (token, _cplx) = m.random_mutate(&mut x, &mut cache, maximum_complexity_mutate);
    //             m.unmutate(&mut x, &mut cache, token);
    //         }
    //     }
    // }
}
