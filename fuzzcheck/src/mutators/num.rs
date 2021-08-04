extern crate self as fuzzcheck;
use fuzzcheck_mutators_derive::make_mutator;
use std::num::FpCategory;

make_mutator! {
    name: FpCategoryMutator,
    default: true,
    type: pub enum FpCategory {
        Nan,
        Infinite,
        Zero,
        Subnormal,
        Normal,
    }
}

// TODO: all non-zero nums, wrapping, etc.
