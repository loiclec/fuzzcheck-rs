use fuzzcheck_mutators::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub struct X(bool);

#[derive(Clone, DefaultMutator)]
pub struct Y {
    x: bool,
}

#[cfg(test)]
mod test {
    use super::*;
    use fuzzcheck_mutators::fuzzcheck_traits::Mutator;
    #[test]
    #[no_coverage]
    fn test_compile() {
        let _m = X::default_mutator();
        let m = Y::default_mutator();

        let (_y, _) = m.random_arbitrary(10.0);
        // assert!(false, "{}", y.x);
    }
}
