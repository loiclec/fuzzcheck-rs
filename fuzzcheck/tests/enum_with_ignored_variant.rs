#![allow(unused_attributes)]
#![feature(coverage_attribute)]

use fuzzcheck::mutators::testing_utilities::test_mutator;
use fuzzcheck::{make_mutator, DefaultMutator, Mutator};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Enum<T> {
    X(bool, bool),
    Y { x: bool, y: T },
}

make_mutator! {
    name: EnumMutator,
    default: true,
    type:
        enum Enum<T> {
            #[ignore_variant]
            X(bool, bool),
            Y {
                x: bool,
                y: T,
            },
        }
}

#[test]
fn test_derived_enum_with_ignored_variant() {
    let mutator = Enum::<u8>::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 100, 100);
    let mutator = <Vec<Enum<u8>>>::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 100, 100);

    let mutator = Enum::<u8>::default_mutator();
    for _ in 0..100 {
        let (v, _) = mutator.random_arbitrary(1000.);
        println!("{v:?}");
    }
}

// this compiles but no value can be produced, the alternation mutator fails with a !mutators.is_empty() assertion
#[derive(Clone, Debug, PartialEq, Eq, Hash, DefaultMutator)]
enum Enum2<T> {
    #[ignore_variant]
    X(bool, bool),
    #[ignore_variant]
    Y { x: bool, y: T },
}
