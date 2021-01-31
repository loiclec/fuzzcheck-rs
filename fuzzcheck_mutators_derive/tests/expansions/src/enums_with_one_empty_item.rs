use fuzzcheck_mutators::DefaultMutator;

#[derive(DefaultMutator, Clone)]
pub enum X {
    A,
}

#[derive(DefaultMutator, Clone)]
pub enum Y {
    A(),
}

#[derive(DefaultMutator, Clone)]
pub enum Z {
    A {},
}

fn _x() {}
