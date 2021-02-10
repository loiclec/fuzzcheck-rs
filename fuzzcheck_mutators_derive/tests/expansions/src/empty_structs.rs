use crate::fuzzcheck_mutators::fuzzcheck_traits::Mutator;
use fuzzcheck_mutators::make_mutator;
use fuzzcheck_mutators::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub struct X;

#[derive(Clone, DefaultMutator)]
pub struct Y {}

#[derive(Clone, DefaultMutator)]
pub struct Z();

#[derive(Clone)]
pub struct XY {
    x: u8,
    y: Vec<u8>,
}

#[make_mutator { name: XYMutator2 , recursive: true , fuzzcheck_mutators_crate: ::fuzzcheck_mutators } ]
pub struct XY {
    x: u8,
    #[mutator(VecMutator<Weak<Self>>)]
    y: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn foo() {
        let m = X::default_mutator();
        let (_, _) = m.random_arbitrary(10.0);

        let mut m = XY::default_mutator();
    }
}
