#![feature(move_ref_pattern)]

extern crate fuzzcheck_mutators;
// #[macro_use]
use fuzzcheck_mutators::fuzzcheck_derive_mutator;
// use fuzzcheck_mutators::HasDefaultMutator;

#[fuzzcheck_derive_mutator]
#[derive(Clone)]
pub enum X {
    A(u8),
    B(u16),
    C,
    D(bool)
}
