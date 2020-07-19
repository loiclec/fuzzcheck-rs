use fuzzcheck_traits::Mutator;

use std::marker::PhantomData;

#[derive(Clone, Default)]
pub struct PhantomDataMutator<T> {
    _p: PhantomData<T>,
}

impl<T> Mutator for PhantomDataMutator<T> {
    type Value = PhantomData<T>;
    type Cache = ();
    type MutationStep = ();
    type UnmutateToken = ();

    fn cache_from_value(&self, _value: &Self::Value) -> Self::Cache {}
    fn initial_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {}
    fn random_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {}

    fn arbitrary(&mut self, _seed: usize, _max_cplx: f64) -> (Self::Value, Self::Cache) {
        (PhantomData, ())
    }

    fn max_complexity(&self) -> f64 {
        0.0
    }

    fn min_complexity(&self) -> f64 {
        0.0
    }

    fn complexity(&self, _value: &Self::Value, _cache: &Self::Cache) -> f64 {
        0.0
    }

    fn mutate(
        &mut self,
        _value: &mut Self::Value,
        _cache: &mut Self::Cache,
        _step: &mut Self::MutationStep,
        _max_cplx: f64,
    ) -> Self::UnmutateToken {
    }

    fn unmutate(&self, _value: &mut Self::Value, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {}
}
