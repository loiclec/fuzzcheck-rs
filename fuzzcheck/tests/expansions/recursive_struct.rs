#![allow(clippy::type_complexity)]
use fuzzcheck::mutators::bool::BoolMutator;
use fuzzcheck::mutators::boxed::BoxMutator;
use fuzzcheck::mutators::option::OptionMutator;
use fuzzcheck::mutators::recursive::{RecurToMutator, RecursiveMutator};
use fuzzcheck::{make_mutator, DefaultMutator, Mutator};

#[derive(Clone, Debug)]
struct S {
    x: bool,
    y: Option<Box<S>>,
}

make_mutator! {
    name: SMutator,
    recursive: true,
    default: true,
    type:
    struct S {
        x: bool,
        #[field_mutator(OptionMutator<Box<S>, BoxMutator<RecurToMutator<SMutator<M0>>>> = { OptionMutator::new(BoxMutator::new(self_.into())) }) ]
        y: Option<Box<S>>,
    }
}

#[derive(Clone)]
pub struct R<T> {
    x: u8,
    y: Option<Box<R<T>>>,
    z: Vec<T>,
}
make_mutator! {
    name: RMutator,
    recursive: true,
    default: true,
    type: // repeat the declaration of E
        pub struct R<T> {
            x: u8,
            // for recursive mutators, it is necessary to indicate *where* the recursion is
            // and use a `RecurToMutator` as the recursive field's mutator
            //                                          M0 is the type parameter for the mutator of the `x` field
            #[field_mutator(OptionMutator<Box<R<T>>, BoxMutator<RecurToMutator<RMutator<T, M0, M2>>>> = { OptionMutator::new(BoxMutator::new(self_.into())) })]
            //                                                                                            self_.into() creates the RecurToMutator
            y: Option<Box<R<T>>>,
            z: Vec<T>
        }
}

mod mutator {}

#[test]
#[coverage(off)]
fn test_compile() {
    let _m = RecursiveMutator::new(|self_| {
        SMutator::new(<bool as DefaultMutator>::default_mutator(), {
            OptionMutator::new(BoxMutator::new(self_.into()))
        })
    });
    let _m: RecursiveMutator<SMutator<BoolMutator>> = S::default_mutator();
    let m = S::default_mutator();
    let (x, _) = m.random_arbitrary(10.0);
    println!("{:?}", x);
}
