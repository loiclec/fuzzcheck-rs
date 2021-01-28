use fuzzcheck_mutators::DefaultMutator;

#[derive(DefaultMutator)]
#[derive(Clone)]
pub enum X {
    A
}

#[derive(DefaultMutator)]
#[derive(Clone)]
pub enum Y {
    A( )
}

#[derive(DefaultMutator)]
#[derive(Clone)]
pub enum Z {
    A { }
}

fn _x() {

}