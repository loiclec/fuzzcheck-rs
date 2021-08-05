use fuzzcheck::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub enum X {
    A,
    B,
}

#[derive(Clone, DefaultMutator)]
pub enum Y {
    A,
    B(),
    C {},
}

#[derive(Clone, DefaultMutator)]
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
    L,
}
