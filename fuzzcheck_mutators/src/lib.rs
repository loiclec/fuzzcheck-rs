#![feature(generic_associated_types)]
#![feature(variant_count)]
#![feature(cmp_min_max_by)]
#![feature(never_type)]
#![feature(int_bits_const)]
#![feature(arc_new_cyclic)]
#![feature(assoc_char_consts)]
#![feature(assoc_char_funcs)]

pub extern crate fastrand;
pub extern crate fuzzcheck_mutators_derive;
pub extern crate fuzzcheck_traits;
pub use fuzzcheck_mutators_derive::*;

mod bool;
mod r#box;
mod dictionary;
mod enums;
mod fixed_len_vector;
mod integer;
mod option;
mod tuples;
mod unit;
mod vector;

/*
List of features required to make string-from-regex mutators:
- [x] add range constraint on generated integer/char
- [x] add unsafe char mutator, but always make it constrained, the constraints are taken from UnicodeClass
- [x] add range constraint on length of generated vectors (at run-time)
- [x] add fixed len vector mutator where each index has a different mutator
- [ ] add constraints on allowed cases of enums (at run-time)
- [ ] have a mutator that wraps multiple mutators of the same type
- [ ] add a MapMutator
- [ ] improve recursive mutators in general, as string-from-regex will depend on it
*/

pub use crate::bool::BoolMutator;
pub use crate::dictionary::DictionaryMutator;
pub use crate::integer::*;
pub use crate::option::OptionMutator;
pub use crate::r#box::BoxMutator;

pub use crate::tuples::{RefTypes, TupleMutator, TupleMutatorWrapper, TupleStructure};

pub use crate::enums::{BasicEnumMutator, BasicEnumStructure};
pub use crate::enums::{Either10, Either11, Either2, Either3, Either4, Either5, Either6, Either7, Either8, Either9};
pub use crate::enums::{
    Enum10PayloadMutator, Enum1PayloadMutator, Enum2PayloadMutator, Enum3PayloadMutator, Enum4PayloadMutator,
    Enum5PayloadMutator, Enum6PayloadMutator, Enum7PayloadMutator, Enum8PayloadMutator, Enum9PayloadMutator,
};
pub use crate::enums::{
    Enum10PayloadStructure, Enum1PayloadStructure, Enum2PayloadStructure, Enum3PayloadStructure, Enum4PayloadStructure,
    Enum5PayloadStructure, Enum6PayloadStructure, Enum7PayloadStructure, Enum8PayloadStructure, Enum9PayloadStructure,
};
pub use crate::tuples::{Tuple1, Tuple10, Tuple2, Tuple3, Tuple4, Tuple5, Tuple6, Tuple7, Tuple8, Tuple9, Wrapped};
pub use crate::tuples::{
    Tuple10Mutator, Tuple1Mutator, Tuple2Mutator, Tuple3Mutator, Tuple4Mutator, Tuple5Mutator, Tuple6Mutator,
    Tuple7Mutator, Tuple8Mutator, Tuple9Mutator,
};

pub use crate::fixed_len_vector::FixedLenVecMutator;
pub use crate::unit::*;
pub use crate::vector::VecMutator;

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
