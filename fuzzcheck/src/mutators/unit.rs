use crate::DefaultMutator;
use crate::Mutator;
use std::any::TypeId;
use std::marker::PhantomData;

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
    type LensPath = !;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        false
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
            Some((self.value.clone(), 0.0))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, _max_cplx: f64) -> (T, f64) {
        (self.value.clone(), 0.0)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        _value: &mut T,
        _cache: &mut Self::Cache,
        _step: &mut Self::MutationStep,
        _max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        None
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, _value: &mut T, _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
        ((), 0.0)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, _value: &mut T, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {}

    #[doc(hidden)]
    #[no_coverage]
    fn lens<'a>(&self, _value: &'a T, _cache: &Self::Cache, _path: &Self::LensPath) -> &'a dyn std::any::Any {
        unreachable!()
    }
    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(&self, _value: &T, _cache: &Self::Cache, _register_path: &mut dyn FnMut(TypeId, Self::LensPath)) {}
    #[doc(hidden)]
    #[no_coverage]
    fn crossover_mutate(
        &self,
        _value: &mut T,
        _cache: &mut Self::Cache,
        _subvalue_provider: &dyn crate::SubValueProvider,
        _max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        ((), 0.0)
    }
}
