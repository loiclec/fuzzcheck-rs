use crate::Mutator;

pub struct ArbitraryOnlyMutator<M> {
    pub mutator: M,
}

impl<M> ArbitraryOnlyMutator<M> {
    pub fn new(mutator: M) -> Self {
        Self { mutator }
    }
}

impl<M, T: Clone> Mutator<T> for ArbitraryOnlyMutator<M>
where
    M: Mutator<T>,
{
    type Cache = ();
    type MutationStep = ();

    type ArbitraryStep = M::ArbitraryStep;
    type UnmutateToken = ();

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    fn validate_value(&self, _value: &T) -> Option<(Self::Cache, Self::MutationStep)> {
        Some(((), ()))
    }

    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    fn complexity(&self, _value: &T, _cache: &Self::Cache) -> f64 {
        1.0
    }

    fn ordered_arbitrary(&self, _step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        Some(self.mutator.random_arbitrary(max_cplx))
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        self.mutator.random_arbitrary(max_cplx)
    }

    fn ordered_mutate(
        &self,
        value: &mut T,
        _cache: &mut Self::Cache,
        _step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        let (mut x, cplx) = self.mutator.random_arbitrary(max_cplx);
        std::mem::swap(value, &mut x);
        Some(((), cplx))
    }

    fn random_mutate(&self, value: &mut T, _cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let (mut x, cplx) = self.mutator.random_arbitrary(max_cplx);
        std::mem::swap(value, &mut x);
        ((), cplx)
    }

    fn unmutate(&self, _value: &mut T, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {}
}
