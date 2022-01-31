use std::any::TypeId;

use crate::DefaultMutator;
use crate::Mutator;

/// Default mutator of `Box<T>`
#[derive(Default)]
pub struct BoxMutator<M> {
    mutator: M,
    rng: fastrand::Rng,
}
impl<M> BoxMutator<M> {
    #[no_coverage]
    pub fn new(mutator: M) -> Self {
        Self {
            mutator,
            rng: fastrand::Rng::new(),
        }
    }
}

pub enum UnmutateToken<T, U> {
    Replace(T),
    Inner(U),
}

impl<T: Clone + 'static, M: Mutator<T>> Mutator<Box<T>> for BoxMutator<M> {
    #[doc(hidden)]
    type Cache = M::Cache;
    #[doc(hidden)]
    type MutationStep = M::MutationStep;
    #[doc(hidden)]
    type ArbitraryStep = M::ArbitraryStep;
    #[doc(hidden)]
    type UnmutateToken = UnmutateToken<T, M::UnmutateToken>;
    #[doc(hidden)]
    type LensPath = M::LensPath;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &Box<T>) -> Option<Self::Cache> {
        self.mutator.validate_value(value)
    }
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, value: &Box<T>, cache: &Self::Cache) -> Self::MutationStep {
        self.mutator.default_mutation_step(value, cache)
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
        if let Some((t, cplx)) = self.mutator.ordered_mutate(value, cache, step, max_cplx) {
            Some((UnmutateToken::Inner(t), cplx))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut Box<T>, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let (t, cplx) = self.mutator.random_mutate(value, cache, max_cplx);
        (UnmutateToken::Inner(t), cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut Box<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateToken::Replace(x) => **value = x,
            UnmutateToken::Inner(t) => self.mutator.unmutate(value, cache, t),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn lens<'a>(&self, value: &'a Box<T>, cache: &'a Self::Cache, path: &Self::LensPath) -> &'a dyn std::any::Any {
        self.mutator.lens(value, cache, path)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(&self, value: &Box<T>, cache: &Self::Cache, register_path: &mut dyn FnMut(TypeId, Self::LensPath)) {
        self.mutator.all_paths(value, cache, register_path)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn crossover_mutate(
        &self,
        value: &mut Box<T>,
        cache: &mut Self::Cache,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        if self.rng.bool() {
            if let Some((subvalue, subcache)) = subvalue_provider
                .get_subvalue(TypeId::of::<T>())
                .and_then(
                    #[no_coverage]
                    |x| x.downcast_ref::<T>(),
                )
                .and_then(
                    #[no_coverage]
                    |v| {
                        self.mutator.validate_value(&v).map(
                            #[no_coverage]
                            |c| (v, c),
                        )
                    },
                )
            {
                let cplx = self.mutator.complexity(&subvalue, &subcache);
                if cplx < max_cplx {
                    let mut swapped = subvalue.clone();
                    std::mem::swap(value.as_mut(), &mut swapped);
                    return (UnmutateToken::Replace(swapped), cplx);
                }
            }
        }
        let (token, cplx) = self
            .mutator
            .crossover_mutate(value.as_mut(), cache, subvalue_provider, max_cplx);
        (UnmutateToken::Inner(token), cplx)
    }
}

impl<T> DefaultMutator for Box<T>
where
    T: DefaultMutator + 'static,
{
    #[doc(hidden)]
    type Mutator = BoxMutator<<T as DefaultMutator>::Mutator>;
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator())
    }
}
