use fuzzcheck_mutators::fuzzcheck_derive_mutator;

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone, Default)]
pub struct X;

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone, Default)]
pub struct Y { }

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone, Default)]
pub struct Z ( );

