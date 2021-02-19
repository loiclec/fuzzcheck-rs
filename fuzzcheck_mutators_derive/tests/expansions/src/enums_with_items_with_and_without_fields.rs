use fuzzcheck_mutators::{DefaultMutator, EnumNPayloadStructure};

#[derive(Clone, DefaultMutator)]
pub enum A {
    X(u8),
}

#[derive(Clone, DefaultMutator)]
pub enum X {
    A(u8),
    B,
}

#[derive(Clone, EnumNPayloadStructure)]
pub enum Y {
    V {
        v: Vec<Option<bool>>,
        w: (),
        x: ::std::collections::HashMap<u8, bool>,
        y: u8,
        z: bool,
    },
    W(bool, bool, bool, bool),
    X(bool),
    Y {
        y: Option<u8>,
    },
    Z(),
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
        let (_alue, _cache): (A, _) = m.random_arbitrary(10.0);

        let m = X::default_mutator();
        let (value, _cache): (X, _) = m.random_arbitrary(10.0);

        match value {
            X::A(_x) => {}
            X::B => {}
        }

        let m = Z::default_mutator();
        let (value, _cache): (Z, _) = m.random_arbitrary(10.0);

        match value {
            _ => {}
        }
    }
}
