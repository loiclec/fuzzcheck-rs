use fuzzcheck::{DefaultMutator, Mutator};
use fuzzcheck_mutators_derive::make_mutator;

#[derive(Clone, DefaultMutator)]
enum NoAssociatedData {
    #[ignore_variant]
    A,
    B,
    #[ignore_variant]
    C,
    D,
    E,
}

#[derive(Clone)]
enum WithIgnore<T> {
    CanMutate(u8),
    CannotMutate(CannotMutate),
    X,
    Y,
    Z,
    A { flag: bool, item: T },
}

#[derive(Clone)]
struct CannotMutate {}

make_mutator! {
    name: WithIgnoreMutator,
    recursive: false,
    default: true,
    type:
        enum WithIgnore<T> {
            CanMutate(u8),
            #[ignore_variant]
            CannotMutate(CannotMutate),
            #[ignore_variant]
            X,
            #[ignore_variant]
            Y,
            #[ignore_variant]
            Z,
            #[ignore_variant]
            A {
                flag: bool,
                item: T
            }
        }
}

#[test]
#[coverage(off)]
fn test_compile() {
    let m = WithIgnore::<bool>::default_mutator();
    let _ = m.random_arbitrary(10.0);
}
