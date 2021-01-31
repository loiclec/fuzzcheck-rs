use crate::fuzzcheck_mutators::fuzzcheck_traits::Mutator;
use fuzzcheck_mutators::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub enum X {
    A(u8),
    B(u8),
}

fn _x() {
    let mut m = X::default_mutator();
    let (_value, _cache): (X, _) = m.random_arbitrary(10.0);
}

// #[derive(DefaultMutator)]
// #[derive(Clone)]
// pub enum Y {
//     Y { y: Option<u8> },
// }
