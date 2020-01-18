extern crate fuzzcheck;
use fuzzcheck::Mutator;

#[derive(Clone)]
pub struct VoidMutator {}

impl Default for VoidMutator {
    fn default() -> Self {
        Self {}
    }
}

impl Mutator for VoidMutator {
    type Value = ();
    type Cache = ();
    type MutationStep = ();
    type UnmutateToken = ();

    fn cache_from_value(&self, _value: &Self::Value) -> Self::Cache {}
    fn mutation_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {}

    fn arbitrary(&self, _seed: usize, _max_cplx: f64) -> (Self::Value, Self::Cache) {
        ((), ())
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
        &self,
        _value: &mut Self::Value,
        _cache: &mut Self::Cache,
        _step: &mut Self::MutationStep,
        _max_cplx: f64,
    ) -> Self::UnmutateToken {
    }

    fn unmutate(&self, _value: &mut Self::Value, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {}
}
