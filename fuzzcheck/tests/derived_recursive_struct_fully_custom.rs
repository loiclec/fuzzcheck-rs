#![allow(unused_attributes)]
#![feature(coverage_attribute)]
use std::marker::PhantomData;

use fuzzcheck::mutators::testing_utilities::test_mutator;
use fuzzcheck::mutators::unit::UnitMutator;
use fuzzcheck::{make_mutator, DefaultMutator};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct SampleStruct<T> {
    // #[field_mutator( <bool as DefaultMutator>::Mutator = { bool::default_mutator() } )]
    x: bool,
    // #[field_mutator( <bool as DefaultMutator>::Mutator = { bool::default_mutator() } )]
    y: bool,
    // #[field_mutator( UnitMutator<PhantomData<T>> = { UnitMutator::new(PhantomData, 0.0) } )]
    _phantom: PhantomData<T>,
}

make_mutator! {
    name: SampleStructMutator,
    recursive: true,
    default: true,
    type:
        struct SampleStruct<T> {
            #[field_mutator( <bool as DefaultMutator>::Mutator = { bool::default_mutator() } )]
            x: bool,
            #[field_mutator( <bool as DefaultMutator>::Mutator = { bool::default_mutator() } )]
            y: bool,
            #[field_mutator( UnitMutator<PhantomData<T>> = { UnitMutator::new(PhantomData, 0.0) } )]
            _phantom: PhantomData<T>,
        }
}

#[test]
fn test_derived_struct() {
    let mutator = SampleStruct::<bool>::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 100, 100);
    let mutator = <Vec<SampleStruct<bool>>>::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 100, 100);
}
