#![feature(never_type)]

pub extern crate fastrand;
pub extern crate fuzzcheck_mutators_derive;
pub extern crate fuzzcheck_traits;
pub use fuzzcheck_mutators_derive::*;

pub mod bool;
pub mod chain;
pub mod dictionary;
pub mod integer;
pub mod option;
pub mod unit;
pub mod vector;

use fuzzcheck_traits::Mutator;
use std::ops::Range;

pub trait DefaultMutator: Clone {
    type Mutator: Mutator<Value = Self> + Default;
    fn default_mutator() -> Self::Mutator;
}

/// Generate a random f64 within the given range
/// The start and end of the range must be finite
/// This is a very naive implementation
#[inline(always)]
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
    (size as f64).log2().round()
}
