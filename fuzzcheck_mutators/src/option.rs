use crate::{self as fuzzcheck_mutators, DefaultMutator};
use fuzzcheck_mutators_derive::fuzzcheck_make_mutator;
use fuzzcheck_traits::Mutator;

#[fuzzcheck_make_mutator(name=OptionMutator)]
pub enum Option<T> {
    Some(T),
    None,
}

impl<T: Clone, M: Mutator<Value = T>> OptionMutator<T, M> {
    pub fn new(inner_mutator: M) -> Self {
        Self {
            Some_0: inner_mutator,
            rng: <_>::default(),
        }
    }
}

impl<T: Clone, M: Mutator<Value = T>> Default for OptionMutator<T, M>
where
    M: Default,
{
    fn default() -> Self {
        Self {
            Some_0: M::default(),
            rng: <_>::default(),
        }
    }
}

impl<T> DefaultMutator for Option<T>
where
    T: DefaultMutator,
{
    type Mutator = OptionMutator<T, T::Mutator>;

    fn default_mutator() -> Self::Mutator {
        Self::Mutator {
            Some_0: T::default_mutator(),
            rng: <_>::default(),
        }
    }
}
