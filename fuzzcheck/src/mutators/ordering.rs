extern crate self as fuzzcheck;

use fuzzcheck_mutators_derive::make_mutator;
use std::cmp::Ordering;

make_mutator! {
    name: OrderingMutator,
    default: true,
    type: pub enum Ordering {
        Less,
        Equal,
        Greater,
    }
}
