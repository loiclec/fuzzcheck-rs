extern crate self as fuzzcheck_mutators;

use fuzzcheck_mutators_derive::make_mutator;
use std::ops::{Bound, Range, RangeFrom, RangeFull, RangeTo, RangeToInclusive};

// TODO: RangeInclusiveMutator with a MapMutator

make_mutator! {
    name: RangeMutator,
    default: true,
    type: pub struct Range<Idx> {
        start: Idx,
        end: Idx,
    }
}

make_mutator! {
    name: RangeFromMutator,
    default: true,
    type: pub struct RangeFrom<Idx> {
        start: Idx,
    }
}

make_mutator! {
    name: RangeToMutator,
    default: true,
    type: pub struct RangeTo<Idx> {
        end: Idx,
    }
}

make_mutator! {
    name: RangeToInclusiveMutator,
    default: true,
    type: pub struct RangeToInclusive<Idx> {
        end: Idx,
    }
}

make_mutator! {
    name: RangeFullMutator,
    default: true,
    type: pub struct RangeFull;
}

make_mutator! {
    name: BoundMutator,
    default: true,
    type: pub enum Bound<T> {
        Included(T),
        Excluded(T),
        Unbounded,
    }
}
