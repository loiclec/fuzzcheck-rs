#![allow(unused_attributes)]
#![feature(coverage_attribute)]
use fuzzcheck::mutators::testing_utilities::test_mutator;
use fuzzcheck::DefaultMutator;

#[derive(Clone, Debug, PartialEq, Eq, Hash, DefaultMutator)]
struct SampleStruct<T, U> {
    x: T,
    y: U,
}

#[test]
fn test_derived_struct() {
    let mutator = SampleStruct::<u8, u8>::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 100, 100);
    let mutator = <Vec<SampleStruct<u8, u8>>>::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 100, 100);
}
