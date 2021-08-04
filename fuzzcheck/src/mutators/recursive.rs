use crate::{Mutator, traits::MutatorWrapper};
use std::rc::{Rc, Weak};

/// The ArbitraryStep that is used for recursive mutators
#[derive(Clone)]
pub enum RecursingArbitraryStep<AS> {
    Default,
    Initialized(AS),
}
impl<AS> Default for RecursingArbitraryStep<AS> {
    #[no_coverage]
    fn default() -> Self {
        Self::Default
    }
}
/**
A wrapper that allows a mutator to call itself recursively.

For example, it is used to provide mutators for types such as:
```ignore
struct S {
    content: T,
    // to mutate this field, a mutator must be able to recursively call itself
    next: Option<Box<S>>
}
```
`RecursiveMutator` is only the top-level type. It must be used in conjuction
with [RecurToMutator](crate::recursive::RecurToMutator) at points of recursion.
For example:
```ignore
use fuzzcheck_mutators::recursive::RecursiveMutator;
let s_mutator = RecursiveMutator::new(|mutator| {
    SMutator {
        content_mutator: TMutator::default(),
        next_mutator: OptionMutator::new(BoxMutator::new(RecurToMutator::from(mutator)))
    }
});
```
*/
pub struct RecursiveMutator<M> {
    pub mutator: Rc<M>,
}
impl<M> RecursiveMutator<M> {
    /// Create a new `RecursiveMutator` using a weak reference to itself.
    #[no_coverage]
    pub fn new(data_fn: impl FnOnce(&Weak<M>) -> M) -> Self {
        Self {
            mutator: Rc::new_cyclic(data_fn),
        }
    }
}

/// A mutator that defers to a weak reference of a [RecursiveMutator](crate::recursive::RecursiveMutator)
pub struct RecurToMutator<M> {
    reference: Weak<M>,
}
impl<M> From<&Weak<M>> for RecurToMutator<M> {
    #[no_coverage]
    fn from(reference: &Weak<M>) -> Self {
        Self {
            reference: reference.clone(),
        }
    }
}

impl<T, M> Mutator<T> for RecurToMutator<M>
where
    M: Mutator<T>,
    T: Clone,
{
    type Cache = <M as Mutator<T>>::Cache;
    type MutationStep = <M as Mutator<T>>::MutationStep;
    type ArbitraryStep = RecursingArbitraryStep<<M as Mutator<T>>::ArbitraryStep>;
    type UnmutateToken = <M as Mutator<T>>::UnmutateToken;

    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        RecursingArbitraryStep::Default
    }

    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<(Self::Cache, Self::MutationStep)> {
        self.reference.upgrade().unwrap().validate_value(value)
    }

    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        std::f64::INFINITY
    }

    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        // should be the min complexity of the mutator
        if let Some(m) = self.reference.upgrade() {
            m.as_ref().min_complexity()
        } else {
            1.0 // not right, but easy hack for now
        }
    }

    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.reference.upgrade().unwrap().complexity(value, cache)
    }

    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        match step {
            RecursingArbitraryStep::Default => {
                let mutator = self.reference.upgrade().unwrap();
                let mut inner_step = mutator.default_arbitrary_step();
                let result = mutator.ordered_arbitrary(&mut inner_step, max_cplx);
                *step = RecursingArbitraryStep::Initialized(inner_step);
                result
            }
            RecursingArbitraryStep::Initialized(inner_step) => self
                .reference
                .upgrade()
                .unwrap()
                .ordered_arbitrary(inner_step, max_cplx),
        }
    }

    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        self.reference.upgrade().unwrap().random_arbitrary(max_cplx)
    }

    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        self.reference
            .upgrade()
            .unwrap()
            .ordered_mutate(value, cache, step, max_cplx)
    }

    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        self.reference.upgrade().unwrap().random_mutate(value, cache, max_cplx)
    }

    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.reference.upgrade().unwrap().unmutate(value, cache, t)
    }
}

impl<M> MutatorWrapper for RecursiveMutator<M> {
    type Wrapped = M;

    #[no_coverage]
    fn wrapped_mutator(&self) -> &Self::Wrapped {
        Rc::as_ref(&self.mutator)
    }
}
