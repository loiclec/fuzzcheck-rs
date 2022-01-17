#![allow(unused_attributes)]
#![feature(no_coverage)]
#![feature(trivial_bounds)]

use fuzzcheck::make_mutator;
use fuzzcheck::mutators::option::OptionMutator;
use fuzzcheck::mutators::recursive::{RecurToMutator, RecursiveMutator};
use fuzzcheck::mutators::testing_utilities::test_mutator;
use fuzzcheck::mutators::vector::VecMutator;
use fuzzcheck::DefaultMutator;

use std::fmt::Debug;

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
            ) = {
                OptionMutator::new(AMutator::new(
                    VecMutator::new(self_.into(), 0..=usize::MAX),
                    <Vec<u64>>::default_mutator(),
                ))
            }]
            a: Option<MutuallyRecursiveA>,
            #[field_mutator(<bool as DefaultMutator>::Mutator = { <bool>::default_mutator() })]
            data: bool
        }
}

/*

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
                    0..=usize::MAX
                )
            }
        )]
        z: Vec<(u8, SampleStruct<T, U>)>,
    }
*/
#[test]
fn test_derived_struct() {
    let mutator = RecursiveMutator::new(|self_| {
        BMutator::new(
            OptionMutator::new(AMutator::new(
                VecMutator::new(self_.into(), 0..=usize::MAX),
                <Vec<u64>>::default_mutator(),
            )),
            bool::default_mutator(),
        )
    });
    // let mutator = SampleStruct::<u8, u8>::default_mutator();
    test_mutator(mutator, 1000., 1000., false, true, 50, 50);
    // let mutator = <Vec<SampleStruct<u8, u8>>>::default_mutator();
    // test_mutator(mutator, 500., 500., false, true, 50, 100);
}
