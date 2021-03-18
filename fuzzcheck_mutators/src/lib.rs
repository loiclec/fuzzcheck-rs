#![feature(generic_associated_types)]
#![feature(auto_traits)]
#![feature(negative_impls)]
#![feature(min_specialization)]
#![feature(variant_count)]
#![feature(int_bits_const)]
#![feature(arc_new_cyclic)]
#![feature(assoc_char_funcs)]
#![feature(array_map)]

pub extern crate fastrand;
pub extern crate fuzzcheck_mutators_derive;
pub extern crate fuzzcheck_traits;
pub use fuzzcheck_mutators_derive::*;

pub mod algebra;
mod alternation;
mod bool;
mod r#box;
mod dictionary;
mod enums;
mod fixed_len_vector;
mod integer;
pub mod map;
mod never;
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
- [x] have a mutator that wraps multiple mutators of different types -> needs to be a proc_macro, called SingleVariantMutator
- [ ] have a mutator that wraps multiple mutators of the same type
    1. important that they are the same type? maybe not
    2. but the mutator acts like the regular enum mutator, tends to stay within same mutator
    except exceptionally to add some randomness
    3. so the SingleVariantMutator I use for string-from-regex should actually contain the types of the
    mutators for the other cases and not NeverMutator
        3.1. an alternative is to compose them with something like:
        mutator_1.or_variant_2(mutator_2).or_variant_4(mutator_4) : AlternativeMutator<SingleVariantMutator<M1, M2, Bottom, M4, Bottom>> ???
        mutator_1.and_variant_2(mutator_2).and_variant_4(mutator_4) : SingleVariantMutator<M1, M2, Bottom, M4, Bottom> ???
        3.2. To do that, I need to add convenience methods to the builder!
        3.3. it may be a good idea to add traits for composing mutators, first by writing the one that
        combines a Bottom and a M and returns a M, but also one that combines two AlternativeMutator or
        a AlternativeMutator and a SingleVariantMutator... need more thoughts into this, but developing
        an algebra of mutators could be nice, especially since it could allow a lot of optimizations
        3.4. then I can get rid of the my custom enum mutators and the EnumStructure traits!!! that's great
- [ ] add a MapMutator
- [ ] improve recursive mutators in general, as generic string-from-grammar depend on them
*/

pub use crate::bool::BoolMutator;
pub use crate::dictionary::DictionaryMutator;
pub use crate::integer::*;
pub use crate::never::*;
pub use crate::option::OptionMutator;
pub use crate::r#box::BoxMutator;
pub use crate::alternation::{AlternationMutator};
pub use crate::tuples::{RefTypes, TupleMutator, TupleMutatorWrapper, TupleStructure};

pub use crate::enums::{BasicEnumMutator, BasicEnumStructure};
// pub use crate::enums::{Either10, Either11, Either2, Either3, Either4, Either5, Either6, Either7, Either8, Either9};
// pub use crate::enums::{
//     Enum10PayloadMutator, Enum1PayloadMutator, Enum2PayloadMutator, Enum3PayloadMutator, Enum4PayloadMutator,
//     Enum5PayloadMutator, Enum6PayloadMutator, Enum7PayloadMutator, Enum8PayloadMutator, Enum9PayloadMutator,
// };
// pub use crate::enums::{
//     Enum10PayloadStructure, Enum1PayloadStructure, Enum2PayloadStructure, Enum3PayloadStructure, Enum4PayloadStructure,
//     Enum5PayloadStructure, Enum6PayloadStructure, Enum7PayloadStructure, Enum8PayloadStructure, Enum9PayloadStructure,
// };
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
