use super::CrossoverStep;
use crate::DefaultMutator;
use crate::Mutator;
use std::any::TypeId;
use std::rc::Rc;

/// Default mutator of `Rc<T>`
#[derive(Default)]
pub struct RcMutator<M> {
    mutator: M,
    rng: fastrand::Rng,
}
impl<M> RcMutator<M> {
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

#[derive(Clone)]
pub struct MutationStep<T, MS> {
    crossover_step: CrossoverStep<T>,
    inner: MS,
}

impl<T: Clone + 'static, M: Mutator<T>> Mutator<Rc<T>> for RcMutator<M> {
    #[doc(hidden)]
    type Cache = M::Cache;
    #[doc(hidden)]
    type MutationStep = MutationStep<T, M::MutationStep>;
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
    fn validate_value(&self, value: &Rc<T>) -> Option<Self::Cache> {
        self.mutator.validate_value(value)
    }
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, value: &Rc<T>, cache: &Self::Cache) -> Self::MutationStep {
        MutationStep {
            crossover_step: CrossoverStep::new(),
            inner: self.mutator.default_mutation_step(value.as_ref(), cache),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn global_search_space_complexity(&self) -> f64 {
        self.mutator.global_search_space_complexity()
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
    fn complexity(&self, value: &Rc<T>, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(Rc<T>, f64)> {
        if let Some((value, cache)) = self.mutator.ordered_arbitrary(step, max_cplx) {
            Some((Rc::new(value), cache))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (Rc<T>, f64) {
        let (value, cache) = self.mutator.random_arbitrary(max_cplx);
        (Rc::new(value), cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut Rc<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if self.rng.usize(..10) == 0 {
            if let Some(result) = step
                .crossover_step
                .get_next_subvalue(subvalue_provider, max_cplx)
                .and_then(
                    #[no_coverage]
                    |x| {
                        self.mutator.validate_value(x).map(
                            #[no_coverage]
                            |c| (x, c),
                        )
                    },
                )
                .map(
                    #[no_coverage]
                    |(replacer, replacer_cache)| {
                        let cplx = self.mutator.complexity(replacer, &replacer_cache);
                        let replacer = replacer.clone();
                        let old_value = value.as_ref().clone();
                        // TODO: something more efficient
                        *value = Rc::new(replacer);
                        (UnmutateToken::Replace(old_value), cplx)
                    },
                )
            {
                return Some(result);
            }
        }
        let mut v = value.as_ref().clone();
        if let Some((t, cplx)) =
            self.mutator
                .ordered_mutate(&mut v, cache, &mut step.inner, subvalue_provider, max_cplx)
        {
            *value = Rc::new(v);
            Some((UnmutateToken::Inner(t), cplx))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut Rc<T>, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let mut v = value.as_ref().clone();
        let (t, cplx) = self.mutator.random_mutate(&mut v, cache, max_cplx);
        *value = Rc::new(v);
        (UnmutateToken::Inner(t), cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut Rc<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateToken::Replace(x) => {
                *value = Rc::new(x);
            }
            UnmutateToken::Inner(t) => {
                let mut v = value.as_ref().clone();
                self.mutator.unmutate(&mut v, cache, t);
                *value = Rc::new(v);
            }
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn lens<'a>(&self, value: &'a Rc<T>, cache: &'a Self::Cache, path: &Self::LensPath) -> &'a dyn std::any::Any {
        self.mutator.lens(value, cache, path)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(
        &self,
        value: &Rc<T>,
        cache: &Self::Cache,
        register_path: &mut dyn FnMut(TypeId, Self::LensPath, f64),
    ) {
        self.mutator.all_paths(value, cache, register_path)
    }
}

impl<T> DefaultMutator for Rc<T>
where
    T: DefaultMutator + 'static,
{
    #[doc(hidden)]
    type Mutator = RcMutator<<T as DefaultMutator>::Mutator>;
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator())
    }
}
