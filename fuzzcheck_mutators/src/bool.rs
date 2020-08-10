use crate::HasDefaultMutator;
use fuzzcheck_traits::Mutator;

#[derive(Default)]
pub struct BoolMutator {
    rng: fastrand::Rng,
}

impl HasDefaultMutator for bool {
    type Mutator = BoolMutator;
    fn default_mutator() -> Self::Mutator {
        <_>::default()
    }   
}

#[derive(Clone)]
pub enum ArbitraryStep {
    Never = 0,
    Once = 1,
    Twice = 2
}
impl Default for ArbitraryStep {
    fn default() -> Self {
        Self::Never
    }
}

impl Mutator for BoolMutator {
    type Value = bool;
    type Cache = ();
    type MutationStep = bool; // true if it has been mutated, false otherwise
    type ArbitraryStep = ArbitraryStep; // None if it has never been called, false if it has been called once, true othewise
    type UnmutateToken = bool;

    fn cache_from_value(&self, _value: &Self::Value) -> Self::Cache {}

    fn initial_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {
        false
    }

    fn ordered_arbitrary(&mut self, step: &mut Self::ArbitraryStep, _max_cplx: f64) -> Option<(Self::Value, Self::Cache)> {
        match step {
            ArbitraryStep::Never => {
                *step = ArbitraryStep::Once;
                Some((false, ()))
            },
            ArbitraryStep::Once => {
                *step = ArbitraryStep::Twice;
                Some((true, ()))
            },
            ArbitraryStep::Twice => {
                None
            },
        }
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

    fn ordered_mutate(
        &mut self,
        value: &mut Self::Value,
        _cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        _max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        if !*step {
            Some(std::mem::replace(value, !*value))
        } else {
            None
        }
    }

    fn random_mutate(
        &mut self,
        value: &mut Self::Value,
        _cache: &mut Self::Cache,
        _max_cplx: f64,
    ) -> Self::UnmutateToken {
        std::mem::replace(value, fastrand::bool())
    }

    fn unmutate(&self, value: &mut Self::Value, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {
        *value = !*value;
    }
}
