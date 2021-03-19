use fuzzcheck_mutators::DefaultMutator;

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

#[cfg(test)]
mod test {
    use super::*;
    use fuzzcheck_mutators::fuzzcheck_traits::Mutator;
    #[test]
    fn test_compile() {
        let m = A::default_mutator();
        let (_alue, _cache, _): (A, _, _) = m.random_arbitrary(10.0);

        let m = X::default_mutator();
        let (value, _cache, _): (X, _, _) = m.random_arbitrary(10.0);

        match value {
            X::A(_x) => {}
            X::B => {}
        }

        let m = Z::default_mutator();
        let (value, _cache, _): (Z, _, _) = m.random_arbitrary(10.0);

        match value {
            _ => {}
        }
    }
}
