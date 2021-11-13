use crate::DefaultMutator;
use crate::Mutator;

/// Default mutator of `Box<T>`
#[derive(Default)]
pub struct BoxMutator<M> {
    mutator: M,
}
impl<M> BoxMutator<M> {
    #[no_coverage]
    pub fn new(mutator: M) -> Self {
        Self { mutator }
    }
}

impl<T: Clone, M: Mutator<T>> Mutator<Box<T>> for BoxMutator<M> {
    #[doc(hidden)]
    type Cache = M::Cache;
    #[doc(hidden)]
    type MutationStep = M::MutationStep;
    #[doc(hidden)]
    type ArbitraryStep = M::ArbitraryStep;
    #[doc(hidden)]
    type UnmutateToken = M::UnmutateToken;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &Box<T>) -> Option<(Self::Cache, Self::MutationStep)> {
        self.mutator.validate_value(value)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, value: &Box<T>, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(Box<T>, f64)> {
        if let Some((value, cache)) = self.mutator.ordered_arbitrary(step, max_cplx) {
            Some((Box::new(value), cache))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (Box<T>, f64) {
        let (value, cache) = self.mutator.random_arbitrary(max_cplx);
        (Box::new(value), cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut Box<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        self.mutator.ordered_mutate(value, cache, step, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut Box<T>, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        self.mutator.random_mutate(value, cache, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut Box<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value, cache, t)
    }
}

impl<T> DefaultMutator for Box<T>
where
    T: DefaultMutator,
{
    #[doc(hidden)]
    type Mutator = BoxMutator<<T as DefaultMutator>::Mutator>;
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator())
    }
}
