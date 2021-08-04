use crate::Mutator;
use fuzzcheck::DefaultMutator;
use fuzzcheck_mutators::boxed::BoxMutator;
use fuzzcheck_mutators::make_mutator;
use fuzzcheck_mutators::recursive::RecurToMutator;

#[test]
#[no_coverage]
fn test_compile() {
    let m = S::default_mutator();
    let (x, _) = m.random_arbitrary(10.0);
    println!("{:?}", x);
}

#[derive(Clone, Debug)]
enum S {
    A(bool),
    B(Box<S>),
}

make_mutator! {
    name: SMutator,
    recursive: true,
    default: true,
    type:
        enum S {
            A(bool),
            B(#[field_mutator(BoxMutator<RecurToMutator<SMutator<M0_0>>> = { BoxMutator::new(self_.into()) }) ] Box<S>),
        }
}
