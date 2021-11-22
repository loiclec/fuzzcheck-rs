use std::sync::Arc;

use crate::DefaultMutator;
use crate::Mutator;

/// Default mutator of `Arc<T>`
#[derive(Default)]
pub struct ArcMutator<M> {
    mutator: M,
}
impl<M> ArcMutator<M> {
    #[no_coverage]
    pub fn new(mutator: M) -> Self {
        Self { mutator }
    }
}

impl<T: Clone, M: Mutator<T>> Mutator<Arc<T>> for ArcMutator<M> {
    #[doc(hidden)]
    type Cache = M::Cache;
    #[doc(hidden)]
    type MutationStep = M::MutationStep;
    #[doc(hidden)]
    type ArbitraryStep = M::ArbitraryStep;
    #[doc(hidden)]
    type UnmutateToken = M::UnmutateToken;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &Arc<T>) -> Option<(Self::Cache, Self::MutationStep)> {
        self.mutator.validate_value(value)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, value: &Arc<T>, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(Arc<T>, f64)> {
        if let Some((value, cache)) = self.mutator.ordered_arbitrary(step, max_cplx) {
            Some((Arc::new(value), cache))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (Arc<T>, f64) {
        let (value, cache) = self.mutator.random_arbitrary(max_cplx);
        (Arc::new(value), cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut Arc<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        let mut v = value.as_ref().clone();
        let res = self.mutator.ordered_mutate(&mut v, cache, step, max_cplx);
        *value = Arc::new(v);
        res
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut Arc<T>, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let mut v = value.as_ref().clone();
        let res = self.mutator.random_mutate(&mut v, cache, max_cplx);
        *value = Arc::new(v);
        res
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut Arc<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        let mut v = value.as_ref().clone();
        self.mutator.unmutate(&mut v, cache, t);
        *value = Arc::new(v);
    }

    #[doc(hidden)]
    type RecursingPartIndex = M::RecursingPartIndex;
    #[doc(hidden)]
    #[no_coverage]
    fn default_recursing_part_index(&self, value: &Arc<T>, cache: &Self::Cache) -> Self::RecursingPartIndex {
        self.mutator.default_recursing_part_index(value, cache)
    }
    #[doc(hidden)]
    #[no_coverage]
    fn recursing_part<'a, V, N>(
        &self,
        parent: &N,
        value: &'a Arc<T>,
        index: &mut Self::RecursingPartIndex,
    ) -> Option<&'a V>
    where
        V: Clone + 'static,
        N: Mutator<V>,
    {
        self.mutator.recursing_part::<V, N>(parent, value, index)
    }
}

impl<T> DefaultMutator for Arc<T>
where
    T: DefaultMutator,
{
    #[doc(hidden)]
    type Mutator = ArcMutator<<T as DefaultMutator>::Mutator>;
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator())
    }
}
