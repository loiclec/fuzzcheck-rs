#![allow(unused_attributes)]
#![feature(coverage_attribute)]

use fuzzcheck::mutators::testing_utilities::test_mutator;
use fuzzcheck::DefaultMutator;

#[derive(Clone, Debug, PartialEq, Eq, Hash, DefaultMutator)]
enum SampleEnum {
    A(u16),
    B,
    C { x: bool, y: bool },
}

#[test]
fn test_derived_enum() {
    let mutator = SampleEnum::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 100, 100);
    let mutator = <Vec<SampleEnum>>::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 100, 100);
}
