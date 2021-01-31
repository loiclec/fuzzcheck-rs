use fuzzcheck_mutators::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub struct X(bool);

#[derive(Clone, DefaultMutator)]
pub struct Y {
    _x: bool,
}

fn _x() {
    let _m = X::default_mutator();
    let _m = Y::default_mutator();
}
