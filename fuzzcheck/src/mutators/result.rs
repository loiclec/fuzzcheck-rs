use std::result::Result;

use fuzzcheck_mutators_derive::make_mutator;
extern crate self as fuzzcheck;

make_mutator! {
    name: ResultMutator,
    default: true,
    type: pub enum Result<T,E> {
        Ok(T),
        Err(E)
    }
}
