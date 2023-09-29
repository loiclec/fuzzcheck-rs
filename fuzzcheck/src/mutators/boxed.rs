use std::any::Any;

use super::CrossoverStep;
use crate::{DefaultMutator, Mutator, CROSSOVER_RATE};

/// Default mutator of `Box<T>`
#[derive(Default)]
pub struct BoxMutator<M> {
    mutator: M,
    rng: fastrand::Rng,
}
impl<M> BoxMutator<M> {
    #[coverage(off)]
    pub fn new(mutator: M) -> Self {
        Self {
            mutator,
            rng: fastrand::Rng::new(),
        }
    }
}
#[derive(Clone)]
pub struct MutationStep<T, MS> {
    crossover_step: CrossoverStep<T>,
    inner: MS,
}

pub enum UnmutateToken<T, U> {
    Replace(T),
    Inner(U),
}

impl<T: Clone + 'static, M: Mutator<T>> Mutator<Box<T>> for BoxMutator<M> {
    #[doc(hidden)]
    type Cache = M::Cache;
    #[doc(hidden)]
    type MutationStep = MutationStep<T, M::MutationStep>;
    #[doc(hidden)]
    type ArbitraryStep = M::ArbitraryStep;
    #[doc(hidden)]
    type UnmutateToken = UnmutateToken<T, M::UnmutateToken>;

    #[doc(hidden)]
    #[coverage(off)]
    fn initialize(&self) {
        self.mutator.initialize();
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn is_valid(&self, value: &Box<T>) -> bool {
        self.mutator.is_valid(value)
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn validate_value(&self, value: &Box<T>) -> Option<Self::Cache> {
        self.mutator.validate_value(value)
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn default_mutation_step(&self, value: &Box<T>, cache: &Self::Cache) -> Self::MutationStep {
        MutationStep {
            crossover_step: CrossoverStep::default(),
            inner: self.mutator.default_mutation_step(value, cache),
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        self.mutator.global_search_space_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn complexity(&self, value: &Box<T>, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(Box<T>, f64)> {
        if let Some((value, cache)) = self.mutator.ordered_arbitrary(step, max_cplx) {
            Some((Box::new(value), cache))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_arbitrary(&self, max_cplx: f64) -> (Box<T>, f64) {
        let (value, cache) = self.mutator.random_arbitrary(max_cplx);
        (Box::new(value), cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_mutate(
        &self,
        value: &mut Box<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if self.rng.u8(..CROSSOVER_RATE) == 0
            && let Some((subvalue, subcplx)) = step.crossover_step.get_next_subvalue(subvalue_provider, max_cplx)
            && self.mutator.is_valid(subvalue) {
            let mut replacer = subvalue.clone();
            std::mem::swap(value.as_mut(), &mut replacer);
            return Some((UnmutateToken::Replace(replacer), subcplx));
        }
        if let Some((t, cplx)) = self
            .mutator
            .ordered_mutate(value, cache, &mut step.inner, subvalue_provider, max_cplx)
        {
            Some((UnmutateToken::Inner(t), cplx))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut Box<T>, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let (t, cplx) = self.mutator.random_mutate(value, cache, max_cplx);
        (UnmutateToken::Inner(t), cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn unmutate(&self, value: &mut Box<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateToken::Replace(x) => **value = x,
            UnmutateToken::Inner(t) => self.mutator.unmutate(value, cache, t),
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, value: &'a Box<T>, cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        self.mutator.visit_subvalues(value, cache, visit)
    }
}

impl<T> DefaultMutator for Box<T>
where
    T: DefaultMutator + 'static,
{
    #[doc(hidden)]
    type Mutator = BoxMutator<<T as DefaultMutator>::Mutator>;
    #[doc(hidden)]
    #[coverage(off)]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator())
    }
}
