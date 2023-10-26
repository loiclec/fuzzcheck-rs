use fuzzcheck::{DefaultMutator, Mutator};

#[derive(Clone, DefaultMutator)]
pub enum A {
    X(u8),
}

#[derive(Clone, DefaultMutator)]
pub enum X {
    A(u8),
    B,
}

#[derive(Clone, DefaultMutator)]
pub enum Z {
    A(u8),
    B(u16),
    C,
    D(bool),
}

#[test]
#[coverage(off)]
fn test_compile() {
    let m = A::default_mutator();
    let (_alue, _cache): (A, _) = m.random_arbitrary(10.0);

    let m = X::default_mutator();
    let (value, _): (X, _) = m.random_arbitrary(10.0);

    match value {
        X::A(_x) => {}
        X::B => {}
    }

    let m = Z::default_mutator();
    let (_value, _): (Z, _) = m.random_arbitrary(10.0);
}
