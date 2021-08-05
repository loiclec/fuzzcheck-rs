/*!
This crate provides a range of [mutators](fuzzcheck_traits::Mutator) that can
be used to run structure-aware fuzz tests using the [fuzzcheck](https://github.com/loiclec/fuzzcheck-rs)
crate. It also provides the [DefaultMutator] trait, which assigns a default mutator
to a type:
```
use fuzzcheck::DefaultMutator;
let mutator = <Vec<Vec<Option<Box<u16>>>>>::default_mutator();
```

The following procedural macros are provided:
- [`#[derive(DefaultMutator)]`](fuzzcheck_mutators_derive::DefaultMutator) creates a mutator for
a non-recursive `struct` or `enum` and makes it its default mutator.
- [`make_mutator! { .. }`](fuzzcheck_mutators_derive::make_mutator) creates a mutator for an arbitrary
`struct` or `enum`. It can be parameterized to do more than what `#[derive(DefaultMutator)]` allows.
- [`make_basic_tuple_mutator!(N)`](fuzzcheck_mutators_derive::make_mutator) creates a mutator for tuples of `N`
elements. For small values of `N`, these mutators are already available in [the `tuples` module](crate::mutators::tuples)

This crate provides [grammar-based string mutators](crate::grammar).
*/

pub use fuzzcheck_mutators_derive::*;

pub mod alternation;
pub mod bool;
pub mod boxed;
pub mod char;
pub mod dictionary;
pub mod either;
pub mod enums;
pub mod fixed_len_vector;
pub mod grammar;
pub mod incremental_map;
pub mod integer;
pub mod integer_within_range;
pub mod map;
pub mod never;
pub mod num;
pub mod option;
pub mod ordering;
pub mod range;
pub mod recursive;
pub mod result;
pub mod tuples;
pub mod unit;
pub mod vector;
pub mod vose_alias;
pub mod wrapper;

use crate::Mutator;
use std::ops::Range;

pub trait DefaultMutator: Clone {
    type Mutator: Mutator<Self>;
    fn default_mutator() -> Self::Mutator;
}

/// Generate a random f64 within the given range
/// The start and end of the range must be finite
/// This is a very naive implementation
#[no_coverage]
fn gen_f64(rng: &fastrand::Rng, range: Range<f64>) -> f64 {
    range.start + rng.f64() * (range.end - range.start)
}

#[must_use]
#[no_coverage]
fn cplxity_to_size(cplx: f64) -> usize {
    let size_f: f64 = 2.0_f64.powf(cplx).round();
    if std::usize::MAX as f64 > size_f {
        size_f as usize
    } else {
        std::usize::MAX
    }
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
        nbr_arbitraries: usize,
        nbr_mutations: usize,
    ) where
        M: Mutator<T>,
        T: Clone + Debug + PartialEq + Eq + Hash,
        M::Cache: Clone + Debug + PartialEq,
    {
        let mut arbitrary_step = m.default_arbitrary_step();

        let mut arbitraries = HashSet::new();
        for _i in 0..nbr_arbitraries {
            if let Some((x, cplx)) = m.ordered_arbitrary(&mut arbitrary_step, maximum_complexity_arbitrary) {
                // assert!(cplx <= maximum_complexity_mutate);
                if avoid_duplicates {
                    let is_new = arbitraries.insert(x.clone());
                    assert!(is_new);
                }
                let (cache, mut mutation_step) = m.validate_value(&x).unwrap();
                let other_cplx = m.complexity(&x, &cache);
                assert!((cplx - other_cplx).abs() < 0.01, "{:.3} != {:.3}", cplx, other_cplx);

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
                        // assert!(cplx <= maximum_complexity_mutate);
                        if avoid_duplicates {
                            let is_new = mutated.insert(x_mut.clone());
                            assert!(is_new);
                        }

                        let validated = m.validate_value(&x_mut).unwrap();
                        let other_cplx = m.complexity(&x_mut, &validated.0);
                        assert!(
                            (cplx - other_cplx).abs() < 0.01,
                            "{:.3} != {:.3} for {:?}",
                            cplx,
                            other_cplx,
                            x_mut
                        );
                        m.unmutate(&mut x_mut, &mut cache_mut, token);
                        assert_eq!(x, x_mut);
                        assert_eq!(cache, cache_mut);
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
            let (x, cplx) = m.random_arbitrary(maximum_complexity_arbitrary);
            let (cache, _) = m.validate_value(&x).unwrap();
            let other_cplx = m.complexity(&x, &cache);
            assert!((cplx - other_cplx).abs() < 0.01, "{:.3} != {:.3}", cplx, other_cplx);
            let mut x_mut = x.clone();
            let mut cache_mut = cache.clone();
            for _j in 0..nbr_mutations {
                let (token, cplx) = m.random_mutate(&mut x_mut, &mut cache_mut, maximum_complexity_mutate);
                let validated = m.validate_value(&x_mut).unwrap();
                let other_cplx = m.complexity(&x_mut, &validated.0);
                assert!((cplx - other_cplx).abs() < 0.01, "{:.3} != {:.3}", cplx, other_cplx);
                m.unmutate(&mut x_mut, &mut cache_mut, token);
                assert_eq!(x, x_mut);
                assert_eq!(cache, cache_mut);
            }
        }
    }
}
