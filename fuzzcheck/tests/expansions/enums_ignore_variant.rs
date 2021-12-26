use fuzzcheck::DefaultMutator;
use fuzzcheck::Mutator;
use fuzzcheck_mutators_derive::make_mutator;

#[derive(Clone)]
enum WithIgnore {
    CanMutate(u8),
    CannotMutate(CannotMutate),
}

#[derive(Clone)]
struct CannotMutate {}

make_mutator! {
    name: WithIgnoreMutator,
    recursive: false,
    default: true,
    type:
        enum WithIgnore {
            CanMutate(u8),
            #[ignore_variant]
            CannotMutate(CannotMutate)
        }
}

#[test]
#[no_coverage]
fn test_compile() {
    let m = WithIgnore::default_mutator();
    let _ = m.random_arbitrary(10.0);
}
