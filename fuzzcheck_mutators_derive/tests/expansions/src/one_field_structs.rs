use fuzzcheck_mutators::fuzzcheck_derive_mutator;

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub struct X(bool);

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub struct Y { _x: bool }


