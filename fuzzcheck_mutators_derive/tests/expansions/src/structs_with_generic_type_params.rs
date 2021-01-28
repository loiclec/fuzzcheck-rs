use fuzzcheck_mutators::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub struct X<T>(T, Vec<T>);

#[derive(Clone, DefaultMutator)]
pub struct Y<T, U> { _x: Option<T>, _y: (T, U) }