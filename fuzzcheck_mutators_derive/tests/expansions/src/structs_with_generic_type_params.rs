use fuzzcheck_mutators::fuzzcheck_derive_mutator;

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub struct X<T>(T, Vec<T>);

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub struct Y<T, U> { _x: Option<T>, _y: (T, U) }