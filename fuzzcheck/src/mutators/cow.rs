use std::borrow::Cow;

use super::map::MapMutator;
use crate::{DefaultMutator, Mutator};

impl<T> DefaultMutator for Cow<'static, T>
where
    T: DefaultMutator + Clone + 'static,
{
    type Mutator = impl Mutator<Cow<'static, T>>;

    #[coverage(off)]
    fn default_mutator() -> Self::Mutator {
        MapMutator::new(
            T::default_mutator(),
            #[coverage(off)]
            |t: &Cow<T>| Some(t.clone().into_owned()),
            #[coverage(off)]
            |t: &T| Cow::Owned(t.clone()),
            #[coverage(off)]
            |_, cplx| cplx,
        )
    }
}
