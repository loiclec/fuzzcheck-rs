use crate::DefaultMutator;
use crate::Mutator;
use std::any::TypeId;

/// Default mutator for `bool`
#[derive(Default)]
pub struct BoolMutator {
    rng: fastrand::Rng,
}

impl DefaultMutator for bool {
    type Mutator = BoolMutator;
    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        <_>::default()
    }
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum ArbitraryStep {
    Never = 0,
    Once = 1,
    Twice = 2,
}
impl Default for ArbitraryStep {
    #[no_coverage]
    fn default() -> Self {
        Self::Never
    }
}

const BOOL_COMPLEXITY: f64 = 1.0;
const INITIAL_MUTATION_STEP: bool = false;

impl Mutator<bool> for BoolMutator {
    #[doc(hidden)]
    type Cache = ();
    #[doc(hidden)]
    type MutationStep = bool;
    #[doc(hidden)]
    type ArbitraryStep = ArbitraryStep;
    #[doc(hidden)]
    type UnmutateToken = bool;
    #[doc(hidden)]
    type LensPath = !;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        <_>::default()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, _value: &bool) -> Option<Self::Cache> {
        Some(())
    }

    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, _value: &bool, _cache: &Self::Cache) -> Self::MutationStep {
        INITIAL_MUTATION_STEP
    }

    #[doc(hidden)]
    #[no_coverage]
    fn global_search_space_complexity(&self) -> f64 {
        1.0
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        BOOL_COMPLEXITY
    }
    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        BOOL_COMPLEXITY
    }
    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, _value: &bool, _cache: &Self::Cache) -> f64 {
        BOOL_COMPLEXITY
    }
    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(bool, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        match step {
            ArbitraryStep::Never => {
                *step = ArbitraryStep::Once;
                Some((false, BOOL_COMPLEXITY))
            }
            ArbitraryStep::Once => {
                *step = ArbitraryStep::Twice;
                Some((true, BOOL_COMPLEXITY))
            }
            ArbitraryStep::Twice => None,
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, _max_cplx: f64) -> (bool, f64) {
        (self.rng.bool(), BOOL_COMPLEXITY)
    }
    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut bool,
        _cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        if !*step {
            *step = !*step;
            Some((std::mem::replace(value, !*value), BOOL_COMPLEXITY))
        } else {
            None
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut bool, _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
        (std::mem::replace(value, !*value), BOOL_COMPLEXITY)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut bool, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = t;
    }

    #[doc(hidden)]
    #[no_coverage]
    fn lens<'a>(&self, _value: &'a bool, _cache: &Self::Cache, _path: &Self::LensPath) -> &'a dyn std::any::Any {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(&self, _value: &bool, _cache: &Self::Cache, _register_path: &mut dyn FnMut(TypeId, Self::LensPath)) {}

    #[doc(hidden)]
    #[no_coverage]
    fn crossover_mutate(
        &self,
        value: &mut bool,
        cache: &mut Self::Cache,
        _subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        self.random_mutate(value, cache, max_cplx)
    }
}
