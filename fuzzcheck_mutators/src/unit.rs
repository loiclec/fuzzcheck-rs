
use std::marker::PhantomData;

use fuzzcheck_traits::Mutator;

use crate::HasDefaultMutator;

pub type VoidMutator = UnitMutator<()>;

impl HasDefaultMutator for () {
    type Mutator = VoidMutator;
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::default()
    }
}

pub type PhantomDataMutator<T> = UnitMutator<PhantomData<T>>;
impl<T> HasDefaultMutator for PhantomData<T> {
    type Mutator = PhantomDataMutator<T>;
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::default()
    }
}


#[derive(Clone)]
pub struct UnitMutator<T> where T: Clone {
    value: T
}

impl<T> UnitMutator<T> where T: Clone {
    pub fn new(value: T) -> Self {
        Self {
            value
        }
    }
}

impl<T> Default for UnitMutator<T> where T: Default + Clone {
    fn default() -> Self {
        Self {
            value: T::default()
        }
    }
}

impl<T> Mutator for UnitMutator<T> where T: Clone {
    type Value = T;
    type Cache = ();
    type MutationStep = ();
    type ArbitraryStep = bool;
    type UnmutateToken = ();

    fn cache_from_value(&self, _value: &Self::Value) -> Self::Cache {}
    
    fn initial_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {}

    fn ordered_arbitrary(&mut self, step: &mut Self::ArbitraryStep, _max_cplx: f64) -> Option<(Self::Value, Self::Cache)> {
        if !*step {
            *step = true;
            Some((self.value.clone(), ()))
        } else {
            None
        }
    }
    fn random_arbitrary(&mut self, _max_cplx: f64) -> (Self::Value, Self::Cache) {
        (self.value.clone(), ())
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

    fn ordered_mutate(
        &mut self,
        _value: &mut Self::Value,
        _cache: &mut Self::Cache,
        _step: &mut Self::MutationStep,
        _max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        None
    }
    fn random_mutate(
        &mut self,
        _value: &mut Self::Value,
        _cache: &mut Self::Cache,
        _max_cplx: f64,
    ) -> Self::UnmutateToken {
        
    }

    fn unmutate(&self, _value: &mut Self::Value, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {}
}
