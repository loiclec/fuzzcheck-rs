use fuzzcheck_mutators::fuzzcheck_derive_mutator;

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub struct X(bool, u8);

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub struct Y { _x: bool, _y: Vec<u8> }