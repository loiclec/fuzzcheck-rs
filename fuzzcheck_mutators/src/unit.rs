use std::marker::PhantomData;

use crate::fuzzcheck_traits::Mutator;

use crate::DefaultMutator;

pub type VoidMutator = UnitMutator<()>;

impl DefaultMutator for () {
    type Mutator = VoidMutator;
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::default()
    }
}

pub type PhantomDataMutator<T> = UnitMutator<PhantomData<T>>;
impl<T> DefaultMutator for PhantomData<T>
where
    T: 'static,
{
    type Mutator = PhantomDataMutator<T>;
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::default()
    }
}

#[derive(Clone)]
pub struct UnitMutator<T>
where
    T: Clone,
{
    value: T,
}

impl<T> UnitMutator<T>
where
    T: Clone,
{
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T> Default for UnitMutator<T>
where
    T: Default + Clone,
{
    fn default() -> Self {
        Self { value: T::default() }
    }
}

impl<T> Mutator<T> for UnitMutator<T>
where
    T: Clone + 'static,
{
    type Cache = ();
    type MutationStep = ();
    type ArbitraryStep = bool;
    type UnmutateToken = ();

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        false
    }

    fn validate_value(&self, _value: &T) -> Option<(Self::Cache, Self::MutationStep)> {
        Some(((), ()))
    }

    fn max_complexity(&self) -> f64 {
        0.0
    }

    fn min_complexity(&self) -> f64 {
        0.0
    }

    fn complexity(&self, _value: &T, _cache: &Self::Cache) -> f64 {
        0.0
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, _max_cplx: f64) -> Option<(T, f64)> {
        if !*step {
            *step = true;
            Some((self.value.clone(), 0.0))
        } else {
            None
        }
    }

    fn random_arbitrary(&self, _max_cplx: f64) -> (T, f64) {
        (self.value.clone(), 0.0)
    }

    fn ordered_mutate(
        &self,
        _value: &mut T,
        _cache: &mut Self::Cache,
        _step: &mut Self::MutationStep,
        _max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        None
    }

    fn random_mutate(&self, _value: &mut T, _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
        ((), 0.0)
    }

    fn unmutate(&self, _value: &mut T, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {}
}
