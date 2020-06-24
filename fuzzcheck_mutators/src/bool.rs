
use fuzzcheck_traits::Mutator;

#[derive(Clone)]
pub struct BoolMutator {}

impl Default for BoolMutator {
    fn default() -> Self {
        BoolMutator {}
    }
}

impl Mutator for BoolMutator {
    type Value = bool;
    type Cache = ();
    type MutationStep = bool; // true if it has been mutated, false otherwise
    type UnmutateToken = ();

    fn cache_from_value(&self, _value: &Self::Value) -> Self::Cache {}

    fn mutation_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {
        false
    }

    fn arbitrary(&mut self, seed: usize, _max_cplx: f64) -> (Self::Value, Self::Cache) {
        let value = seed % 2 == 0;
        (value, ())
    }

    fn max_complexity(&self) -> f64 {
        1.0
    }

    fn min_complexity(&self) -> f64 {
        1.0
    }

    fn complexity(&self, _value: &Self::Value, _cache: &Self::Cache) -> f64 {
        1.0
    }

    fn mutate(
        &mut self,
        value: &mut Self::Value,
        _cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        _max_cplx: f64,
    ) -> Self::UnmutateToken {
        *value = !*value;
        *step = true;
    }

    fn unmutate(&self, value: &mut Self::Value, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {
        *value = !*value;
    }
}
