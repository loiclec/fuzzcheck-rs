use fuzzcheck_traits::Mutator;


#[derive(Default)]
pub struct BoolMutator {
    rng: fastrand::Rng,
}

impl Mutator for BoolMutator {
    type Value = bool;
    type Cache = ();
    type MutationStep = bool; // true if it has been mutated, false otherwise
    type UnmutateToken = ();

    fn cache_from_value(&self, _value: &Self::Value) -> Self::Cache {}

    fn initial_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {
        false
    }
    fn random_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {
        false
    }

    fn ordered_arbitrary(&mut self, seed: usize, _max_cplx: f64) -> Option<(Self::Value, Self::Cache)> {
        if seed == 0 { Some((true, ())) } else if seed == 1 { Some((false, ())) } else { None }
    }
    fn random_arbitrary(&mut self, _max_cplx: f64) -> (Self::Value, Self::Cache) {
        (self.rng.bool(), ())
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
    ) -> Option<Self::UnmutateToken> {
        if !*step {
            *value = !*value;
            Some(())
        } else {
            None
        }
    }

    fn unmutate(&self, value: &mut Self::Value, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {
        *value = !*value;
    }
}
