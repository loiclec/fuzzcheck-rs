use crate::fuzzcheck_mutators::fuzzcheck_traits::Mutator;
use fuzzcheck_mutators::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub struct X;

#[derive(Clone, DefaultMutator)]
pub struct Y {}

#[derive(Clone, DefaultMutator)]
pub struct Z();

fn _x() {
    let mut m = X::default_mutator();
    let (_, _) = m.random_arbitrary(10.0);
}
