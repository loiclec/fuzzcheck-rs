use fuzzcheck_mutators::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub enum X {
    A(u8, bool),
}

#[derive(Clone, DefaultMutator)]
pub enum Y {
    Y { y: Option<u8>, z: () },
}
