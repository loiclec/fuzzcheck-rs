use fuzzcheck_mutators::{DefaultMutator, EnumNPayloadStructure};
use crate::fuzzcheck_mutators::fuzzcheck_traits::Mutator;

#[derive(Clone, DefaultMutator)]
pub enum A {
    X(u8)
}

#[derive(Clone, DefaultMutator)]
pub enum X {
    A(u8),
    B,
}

#[derive(Clone, EnumNPayloadStructure)]
pub enum Y {
    V { v: Vec<Option<bool>>, w: (), x: ::std::collections::HashMap<u8, bool>, y: u8, z: bool },
    W(bool, bool, bool, bool),
    X(bool),
    Y { y: Option<u8> },
    Z (),
}

#[derive(Clone, DefaultMutator)]
pub enum Z {
    A(u8),
    B(u16),
    C,
    D(bool)
}

fn _x() {
    let mut m = A::default_mutator();
    let (_alue, _cache): (A, _) = m.random_arbitrary(10.0);

    let mut m = X::default_mutator();
    let (value, _cache): (X, _) = m.random_arbitrary(10.0);

    match value {
        X::A(_x) => { }
        X::B => {}
    }

    let mut m = Z::default_mutator();
    let (value, _cache): (Z, _) = m.random_arbitrary(10.0);

    match value {
        _ => { }
    }
}
