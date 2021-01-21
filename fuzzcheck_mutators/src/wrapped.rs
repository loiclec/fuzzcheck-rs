use fuzzcheck_traits::Mutator;
use std::marker::PhantomData;

pub trait WrappedStructure {
    type Wrapped;
    fn get_wrapped(&self) -> &Self::Wrapped;
    fn get_wrapped_mut(&mut self) -> &mut Self::Wrapped;
    fn new(wrapped: Self::Wrapped) -> Self;
}

pub struct WrappedMutator<T: Clone, M>
where
    M: Mutator<T>,
{
    pub mutator: M,
    _phantom: PhantomData<T>,
}
impl<T: Clone, M> WrappedMutator<T, M>
where
    M: Mutator<T>,
{
    pub fn new(mutator: M) -> Self {
        Self {
            mutator,
            _phantom: PhantomData,
        }
    }
}

impl<T: Clone, U: Clone, M> Mutator<U> for WrappedMutator<T, M>
where
    U: WrappedStructure<Wrapped = T>,
    M: Mutator<T>,
{
    type Cache = M::Cache;
    type MutationStep = M::MutationStep;
    type ArbitraryStep = M::ArbitraryStep;
    type UnmutateToken = M::UnmutateToken;

    fn cache_from_value(&self, value: &U) -> Self::Cache {
        self.mutator.cache_from_value(value.get_wrapped())
    }

    fn initial_step_from_value(&self, value: &U) -> Self::MutationStep {
        self.mutator.initial_step_from_value(value.get_wrapped())
    }

    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    fn complexity(&self, value: &U, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value.get_wrapped(), cache)
    }

    fn ordered_arbitrary(&mut self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(U, Self::Cache)> {
        self.mutator
            .ordered_arbitrary(step, max_cplx)
            .map(|(v, c)| (U::new(v), c))
    }

    fn random_arbitrary(&mut self, max_cplx: f64) -> (U, Self::Cache) {
        let (v, c) = self.mutator.random_arbitrary(max_cplx);
        (U::new(v), c)
    }

    fn ordered_mutate(
        &mut self,
        value: &mut U,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        self.mutator
            .ordered_mutate(value.get_wrapped_mut(), cache, step, max_cplx)
    }

    fn random_mutate(&mut self, value: &mut U, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        self.mutator.random_mutate(value.get_wrapped_mut(), cache, max_cplx)
    }

    fn unmutate(&self, value: &mut U, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value.get_wrapped_mut(), cache, t)
    }
}
