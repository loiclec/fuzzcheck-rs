use fuzzcheck_mutators_derive::make_mutator;
use std::result::Result;
extern crate self as fuzzcheck_mutators;

make_mutator! {
    name: ResultMutator,
    default: true,
    type: pub enum Result<T,E> {
        Ok(T),
        Err(E)
    }
}
