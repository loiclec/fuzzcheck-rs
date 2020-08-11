use fuzzcheck_mutators::fuzzcheck_derive_mutator;

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub enum X {
    A
}

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub enum Y {
    A( )
}

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub enum Z {
    A { }
}
