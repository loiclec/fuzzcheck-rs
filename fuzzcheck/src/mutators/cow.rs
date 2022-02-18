use std::borrow::Cow;

use super::map::MapMutator;
use crate::{DefaultMutator, Mutator};

impl<T> DefaultMutator for Cow<'static, T>
where
    T: DefaultMutator + Clone + 'static,
{
    type Mutator = impl Mutator<Cow<'static, T>>;

    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        MapMutator::new(
            T::default_mutator(),
            #[no_coverage]
            |t: &Cow<T>| Some(t.clone().into_owned()),
            #[no_coverage]
            |t: &T| Cow::Owned(t.clone()),
            #[no_coverage]
            |_, cplx| cplx,
        )
    }
}
