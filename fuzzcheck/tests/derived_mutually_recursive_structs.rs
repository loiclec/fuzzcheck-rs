#![allow(unused_attributes)]
#![feature(coverage_attribute)]
// #![feature(trivial_bounds)]

use std::fmt::Debug;

use fuzzcheck::mutators::option::OptionMutator;
use fuzzcheck::mutators::recursive::RecurToMutator;
use fuzzcheck::mutators::testing_utilities::test_mutator;
use fuzzcheck::mutators::vector::VecMutator;
use fuzzcheck::{make_mutator, DefaultMutator};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MutuallyRecursiveA {
    b: Vec<MutuallyRecursiveB>,
    data: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MutuallyRecursiveB {
    a: Option<MutuallyRecursiveA>,
    data: bool,
}

make_mutator! {
    name: AMutator,
    recursive: true,
    default: false,
    type:
        struct MutuallyRecursiveA {
            // #[field_mutator(VecMutator<MutuallyRecursiveB, <MutuallyRecursiveB as DefaultMutator>::Mutator> = { MutuallyRecursiveB::default_mutator() })]
            b: Vec<MutuallyRecursiveB>,
            #[field_mutator(<Vec<u64> as DefaultMutator>::Mutator = { <Vec<u64>>::default_mutator() })]
            data: Vec<u64>,
        }
}

make_mutator! {
    name: BMutator,
    recursive: true,
    default: true,
    type:
        struct MutuallyRecursiveB {
            #[field_mutator(
                OptionMutator<MutuallyRecursiveA, AMutator<VecMutator<MutuallyRecursiveB, RecurToMutator<BMutator>>>>
             = {
                OptionMutator::new(AMutator::new(
                    VecMutator::new(self_.into(), 0..=usize::MAX),
                    <Vec<u64>>::default_mutator(),
                ))
            })]
            a: Option<MutuallyRecursiveA>,
            #[field_mutator(<bool as DefaultMutator>::Mutator = { <bool>::default_mutator() })]
            data: bool
        }
}

#[test]
fn test_derived_struct() {
    let mutator = MutuallyRecursiveB::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 50, 50);
}
