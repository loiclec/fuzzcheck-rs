#![feature(generic_associated_types)]
#![feature(variant_count)]
#![feature(arc_new_cyclic)]
#![feature(trivial_bounds)]

pub extern crate fastrand;
pub extern crate fuzzcheck_mutators_derive;

#[cfg(feature = "compile_fuzzcheck_traits")]
pub extern crate fuzzcheck_traits;

#[cfg(feature = "fuzzcheck_traits_through_fuzzcheck")]
pub use fuzzcheck::fuzzcheck_traits;

pub use fuzzcheck_mutators_derive::*;

pub mod alternation;
pub mod bool;
pub mod boxed;
pub mod dictionary;
pub mod either;
pub mod enums;
pub mod fixed_len_vector;
pub mod grammar;
pub mod integer;
pub mod never;
pub mod option;
pub mod recursive;
pub mod tuples;
pub mod unit;
pub mod vector;
pub mod vose_alias;

use crate::fuzzcheck_traits::Mutator;
use std::ops::Range;

pub trait DefaultMutator: Clone {
    type Mutator: Mutator<Self>;
    fn default_mutator() -> Self::Mutator;
}

/// Generate a random f64 within the given range
/// The start and end of the range must be finite
/// This is a very naive implementation
fn gen_f64(rng: &fastrand::Rng, range: Range<f64>) -> f64 {
    range.start + rng.f64() * (range.end - range.start)
}

#[must_use]
fn cplxity_to_size(cplx: f64) -> usize {
    let size_f: f64 = 2.0_f64.powf(cplx).round();
    if std::usize::MAX as f64 > size_f {
        size_f as usize
    } else {
        std::usize::MAX
    }
}
#[must_use]
fn size_to_cplxity(size: usize) -> f64 {
    (usize::BITS - (size.saturating_sub(1)).leading_zeros()) as f64
}

#[cfg(test)]
mod test {
    use crate::size_to_cplxity;

    #[test]
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
