use fuzzcheck_mutators_derive::make_mutator;
extern crate self as fuzzcheck;

make_mutator! {
    name: OptionMutator,
    default: true,
    type: pub enum Option<T> {
        Some(T),
        None,
    }
}
