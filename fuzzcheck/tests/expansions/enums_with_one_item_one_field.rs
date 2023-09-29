use fuzzcheck::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub enum X {
    A(u8),
}

#[cfg(test)]
mod test {
    use fuzzcheck::Mutator;

    use super::*;
    #[test]
    #[coverage(off)]
    fn test_compile() {
        let m = X::default_mutator();
        let (_value, _): (X, _) = m.random_arbitrary(10.0);
    }
}
