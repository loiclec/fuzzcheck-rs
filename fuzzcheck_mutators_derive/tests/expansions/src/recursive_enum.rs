use fuzzcheck_mutators::fuzzcheck_traits::RecurToMutator;
use fuzzcheck_mutators::make_mutator;
use fuzzcheck_mutators::BoxMutator;

#[cfg(test)]
mod test {
    use super::*;
    use fuzzcheck_mutators::fuzzcheck_traits::Mutator;
    use fuzzcheck_mutators::DefaultMutator;
    #[test]
    fn test_compile() {
        let m = S::default_mutator();
        let (x, _, _) = m.random_arbitrary(10.0);
        println!("{:?}", x);
    }
}

#[derive(Clone, Debug)]
enum S {
    A(bool),
    B(Box<S>),
}

#[make_mutator(name: SMutator, recursive: true, default: true)]
enum S {
    A(bool),
    B(#[field_mutator(BoxMutator<S, RecurToMutator<SMutator<M0_0>>> = { BoxMutator::new(self_.into()) }) ] Box<S>),
}

// #[make_mutator(name: SMutator, recursive: true, default: true)]
// struct S {
//     x: bool,
//     #[field_mutator(OptionMutator<Box<S>, BoxMutator<S, RecurToMutator<SMutator<M0>>>> = { OptionMutator::new(BoxMutator::new(self_.into())) }) ]
//     y: Option<Box<S>>,
// }
