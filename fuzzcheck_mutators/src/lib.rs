#![feature(generic_associated_types)]

pub extern crate fastrand;
pub extern crate fuzzcheck_mutators_derive;
pub extern crate fuzzcheck_traits;
pub use fuzzcheck_mutators_derive::*;

mod bool;
// mod chain;
mod dictionary;
mod integer;
// // mod option;
mod enums;
mod tuples;
mod unit;
mod vector;
mod wrapped;

pub use crate::bool::BoolMutator;
pub use crate::dictionary::DictionaryMutator;
pub use crate::integer::*;

pub use crate::tuples::{RefTypes, TupleMutator, TupleMutatorWrapper, TupleStructure};

// pub use crate::option::OptionMutator;
pub use crate::tuples::{Tuple10, Tuple2, Tuple3, Tuple4, Tuple5, Tuple6, Tuple7, Tuple8, Tuple9};
pub use crate::tuples::{
    Tuple10Mutator, Tuple2Mutator, Tuple3Mutator, Tuple4Mutator, Tuple5Mutator, Tuple6Mutator, Tuple7Mutator,
    Tuple8Mutator, Tuple9Mutator,
};
pub use crate::unit::*;
pub use crate::vector::VecMutator;
// pub use crate::wrapped::{WrappedMutator, WrappedStructure};

use fuzzcheck_traits::Mutator;
use std::ops::Range;

pub trait DefaultMutator: Clone {
    type Mutator: Mutator<Self>;
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
