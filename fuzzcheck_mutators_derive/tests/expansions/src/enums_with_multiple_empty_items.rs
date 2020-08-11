use fuzzcheck_mutators::fuzzcheck_derive_mutator;

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub enum X {
    A,
    B
}

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub enum Y {
    A,
    B (),
    C {}
}

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub enum Z {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L
}
