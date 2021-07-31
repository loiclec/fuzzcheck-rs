use fuzzcheck_mutators::make_mutator;
use fuzzcheck_mutators::recursive::{RecurToMutator, RecursiveMutator};
use fuzzcheck_mutators::DefaultMutator;
use fuzzcheck_mutators::{bool::BoolMutator, fuzzcheck_traits::Mutator};
use fuzzcheck_mutators::{boxed::BoxMutator, option::OptionMutator};
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
        #[field_mutator(OptionMutator<Box<S>, BoxMutator<S, RecurToMutator<SMutator<M0>>>> = { OptionMutator::new(BoxMutator::new(self_.into())) }) ]
        y: Option<Box<S>>,
    }
}

#[test]
#[no_coverage] fn test_compile() {
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
