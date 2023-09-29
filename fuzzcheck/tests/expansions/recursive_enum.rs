use fuzzcheck::mutators::boxed::BoxMutator;
use fuzzcheck::mutators::recursive::RecurToMutator;
use fuzzcheck::{make_mutator, DefaultMutator, Mutator};

#[test]
#[coverage(off)]
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
