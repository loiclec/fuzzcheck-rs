#![allow(unused_attributes)]
#![feature(coverage_attribute)]

use fuzzcheck::mutators::testing_utilities::test_mutator;
use fuzzcheck::DefaultMutator;

#[derive(Clone, Debug, PartialEq, Eq, Hash, DefaultMutator)]
struct S<T, const M: usize, const N: usize = 8> {
    x: [T; N],
    y: [bool; M],
}

#[test]
fn test_const_generics_mutator() {
    let mutator = S::<u8, 2>::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 100, 100);
}
