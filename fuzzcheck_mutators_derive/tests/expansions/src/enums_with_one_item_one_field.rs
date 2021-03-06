use fuzzcheck_mutators::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub enum X {
    A(u8),
}

#[cfg(test)]
mod test {
    use super::*;
    use fuzzcheck_mutators::fuzzcheck_traits::Mutator;
    #[test]
    fn test_compile() {
        let m = X::default_mutator();
        let (_value, _cache): (X, _) = m.random_arbitrary(10.0);
    }
}
// #[derive(DefaultMutator)]
// #[derive(Clone)]
// pub enum Y {
//     Y { y: Option<u8> },
// }
