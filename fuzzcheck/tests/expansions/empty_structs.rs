use fuzzcheck::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub struct X;

#[derive(Clone, DefaultMutator)]
pub struct Y {}

#[derive(Clone, DefaultMutator)]
pub struct Z();
