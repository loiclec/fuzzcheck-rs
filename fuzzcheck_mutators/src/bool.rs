use crate::fuzzcheck_traits::Mutator;
use crate::DefaultMutator;

#[derive(Default)]
pub struct BoolMutator {
    rng: fastrand::Rng,
}

impl DefaultMutator for bool {
    type Mutator = BoolMutator;
    fn default_mutator() -> Self::Mutator {
        <_>::default()
    }
}

#[derive(Clone)]
pub enum ArbitraryStep {
    Never = 0,
    Once = 1,
    Twice = 2,
}
impl Default for ArbitraryStep {
    fn default() -> Self {
        Self::Never
    }
}

const INITIAL_MUTATION_STEP: bool = false;

impl Mutator<bool> for BoolMutator {
    type Cache = ();
    type MutationStep = bool;
    type ArbitraryStep = ArbitraryStep;
    type UnmutateToken = bool;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        <_>::default()
    }

    fn max_complexity(&self) -> f64 {
        1.0
    }

    fn min_complexity(&self) -> f64 {
        1.0
    }

    fn validate_value(&self, _value: &bool) -> Option<(Self::Cache, Self::MutationStep)> {
        Some(((), INITIAL_MUTATION_STEP))
    }

    fn complexity(&self, _value: &bool, _cache: &Self::Cache) -> f64 {
        1.0
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(bool, Self::Cache)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        match step {
            ArbitraryStep::Never => {
                *step = ArbitraryStep::Once;
                Some((false, ()))
            }
            ArbitraryStep::Once => {
                *step = ArbitraryStep::Twice;
                Some((true, ()))
            }
            ArbitraryStep::Twice => None,
        }
    }

    fn random_arbitrary(&self, _max_cplx: f64) -> (bool, Self::Cache) {
        (self.rng.bool(), ())
    }

    fn ordered_mutate(
        &self,
        value: &mut bool,
        _cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        if !*step {
            *step = !*step;
            Some(std::mem::replace(value, !*value))
        } else {
            None
        }
    }

    fn random_mutate(&self, value: &mut bool, _cache: &mut Self::Cache, _max_cplx: f64) -> Self::UnmutateToken {
        std::mem::replace(value, !*value)
    }

    fn unmutate(&self, value: &mut bool, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = t;
    }
}
