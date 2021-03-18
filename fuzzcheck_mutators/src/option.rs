use super::fuzzcheck_mutators_derive::make_mutator;
extern crate self as fuzzcheck_mutators;

#[make_mutator(name: OptionMutator, default: true)]
pub enum Option<T> {
    Some(T),
    None,
}

// crate::make_single_variant_mutator! {
//     pub enum Option<T> {
//         Some(T),
//         None,
//     }
// }
