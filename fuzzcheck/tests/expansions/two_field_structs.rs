use fuzzcheck::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub struct X<T, U: Default + Clone = Vec<T>>(bool, u8, T, U, u8, u8, u8, u8, u8, u8);

#[derive(Clone, DefaultMutator)]
pub struct Y {
    #[field_mutator(<bool as DefaultMutator>::Mutator)]
    _x: bool,
    _y: Vec<X<u8>>,
}

#[coverage(off)]
fn _x() {
    let _x = X::<u8, Vec<u64>>::default_mutator();
    let _y = Y::default_mutator();
}
