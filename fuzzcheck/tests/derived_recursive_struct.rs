#![allow(unused_attributes)]
#![allow(clippy::type_complexity)]
#![feature(no_coverage)]

use fuzzcheck::make_mutator;
use fuzzcheck::mutators::boxed::BoxMutator;
use fuzzcheck::mutators::integer::U8Mutator;
use fuzzcheck::mutators::option::OptionMutator;
use fuzzcheck::mutators::recursive::RecurToMutator;
use fuzzcheck::mutators::testing_utilities::test_mutator;
use fuzzcheck::mutators::tuples::{Tuple2, Tuple2Mutator, TupleMutatorWrapper};
use fuzzcheck::mutators::vector::VecMutator;
use fuzzcheck::DefaultMutator;

use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct SampleStruct<T, U> {
    w: Option<Box<SampleStruct<T, U>>>,
    x: T,
    y: U,
    z: Vec<(u8, SampleStruct<T, U>)>,
}

make_mutator! {
    name: SampleStructMutator,
    recursive: true,
    default: true,
    type:
    struct SampleStruct<T, U> {
        #[field_mutator(
            OptionMutator<Box<SampleStruct<T, U>>, BoxMutator<RecurToMutator<SampleStructMutator<T, U, M1, M2>>>> = {
                OptionMutator::new(BoxMutator::new(self_.into()))
            }
        )]
        w: Option<Box<SampleStruct<T, U>>>,
        x: T,
        y: U,
        #[field_mutator(
            VecMutator<
                (u8, SampleStruct<T, U>),
                TupleMutatorWrapper<
                    Tuple2Mutator<
                        U8Mutator,
                        RecurToMutator<
                            SampleStructMutator<T, U, M1, M2>
                        >
                    >,
                    Tuple2<u8, SampleStruct<T, U>>
                >
            > = {
                VecMutator::new(
                    TupleMutatorWrapper::new(
                        Tuple2Mutator::new(
                            u8::default_mutator(),
                            self_.into()
                        )
                    ),
                    0..=usize::MAX,
                    true
                )
            }
        )]
        z: Vec<(u8, SampleStruct<T, U>)>,
    }
}

#[test]
fn test_derived_struct() {
    let mutator = SampleStruct::<u8, u8>::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 50, 50);
    let mutator = <Vec<SampleStruct<u8, u8>>>::default_mutator();
    test_mutator(mutator, 500., 500., false, true, 50, 100);
}
