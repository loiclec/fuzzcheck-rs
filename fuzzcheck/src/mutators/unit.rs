use std::any::Any;
use std::marker::PhantomData;

use crate::{DefaultMutator, Mutator};

pub type VoidMutator = UnitMutator<()>;

impl DefaultMutator for () {
    type Mutator = VoidMutator;
    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new((), 0.0)
    }
}

pub type PhantomDataMutator<T> = UnitMutator<PhantomData<T>>;
impl<T> DefaultMutator for PhantomData<T>
where
    T: 'static,
{
    type Mutator = PhantomDataMutator<T>;
    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(PhantomData, 0.0)
    }
}

#[derive(Clone)]
pub struct UnitMutator<T>
where
    T: Clone,
{
    value: T,
    complexity: f64,
}

impl<T> UnitMutator<T>
where
    T: Clone,
{
    #[no_coverage]
    pub fn new(value: T, complexity: f64) -> Self {
        Self { value, complexity }
    }
}

impl<T> Mutator<T> for UnitMutator<T>
where
    T: Clone + 'static,
{
    #[doc(hidden)]
    type Cache = ();
    #[doc(hidden)]
    type MutationStep = ();
    #[doc(hidden)]
    type ArbitraryStep = bool;
    #[doc(hidden)]
    type UnmutateToken = ();

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        false
    }

    #[doc(hidden)]
    #[no_coverage]
    fn is_valid(&self, _value: &T) -> bool {
        true
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, _value: &T) -> Option<Self::Cache> {
        Some(())
    }

    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, _value: &T, _cache: &Self::Cache) -> Self::MutationStep {}

    #[doc(hidden)]
    #[no_coverage]
    fn global_search_space_complexity(&self) -> f64 {
        0.0
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.complexity
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.complexity
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, _value: &T, _cache: &Self::Cache) -> f64 {
        self.complexity
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, _max_cplx: f64) -> Option<(T, f64)> {
        if !*step {
            *step = true;
            Some((self.value.clone(), self.complexity))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, _max_cplx: f64) -> (T, f64) {
        (self.value.clone(), self.complexity)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        _value: &mut T,
        _cache: &mut Self::Cache,
        _step: &mut Self::MutationStep,
        _subvalue_provider: &dyn crate::SubValueProvider,
        _max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        None
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, _value: &mut T, _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
        ((), self.complexity)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, _value: &mut T, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {}

    #[doc(hidden)]
    #[no_coverage]
    fn visit_subvalues<'a>(&self, _value: &'a T, _cache: &'a Self::Cache, _visit: &mut dyn FnMut(&'a dyn Any, f64)) {}
}
