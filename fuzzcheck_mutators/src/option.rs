use super::fuzzcheck_mutators_derive::make_mutator;

#[make_mutator { fuzzcheck_mutators_crate: crate }]
pub enum Option<T> {
    Some(T),
    None,
}
