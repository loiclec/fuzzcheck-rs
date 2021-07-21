use fuzzcheck_mutators::fuzzcheck_traits::Mutator;
use fuzzcheck_mutators::{DefaultMutator, U8Mutator, VecMutator};

#[derive(Clone, Debug, DefaultMutator)]
pub enum X<T> {
    A(T),
    B(Vec<T>),
}

#[test]
fn test_compile() {
    // let m = X::<Vec<u8>>::default_mutator();
    // let (value, _cache): (X<Vec<u8>>, _) = m.random_arbitrary(10.0);

    // match &value {
    //     X::A(_x) => {}
    //     X::B(_y) => {}
    // }
    // println!("{:?}", value);
    // let m = VecMutator::new(VecMutator::new(U8Mutator::default(), 2..=1000), 4..=10);
    let m = Vec::<Vec<u8>>::default_mutator();

    println!("{}", m.min_complexity());
    let (value, _cache): (Vec<Vec<u8>>, _) = m.random_arbitrary(1000.0);
    println!("{:?} {}", value, value.len());
}
