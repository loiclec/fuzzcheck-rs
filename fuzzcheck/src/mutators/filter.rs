use crate::Mutator;

pub struct FilterMutator<M, F> {
    pub mutator: M,
    pub filter: F,
}
impl<T, M, F> Mutator<T> for FilterMutator<M, F>
where
    M: Mutator<T>,
    T: Clone + 'static,
    F: Fn(&T) -> bool,
    Self: 'static,
{
    #[doc(hidden)]
    type Cache = <M as Mutator<T>>::Cache;
    #[doc(hidden)]
    type MutationStep = <M as Mutator<T>>::MutationStep;
    #[doc(hidden)]
    type ArbitraryStep = <M as Mutator<T>>::ArbitraryStep;
    #[doc(hidden)]
    type UnmutateToken = <M as Mutator<T>>::UnmutateToken;
    #[doc(hidden)]
    type LensPath = M::LensPath;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        let x = self.mutator.validate_value(value);
        if x.is_some() && (self.filter)(value) == false {
            None
        } else {
            x
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        self.mutator.default_mutation_step(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn global_search_space_complexity(&self) -> f64 {
        self.mutator.global_search_space_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        let x = self.mutator.ordered_arbitrary(step, max_cplx);
        if let Some(x) = x {
            if (self.filter)(&x.0) {
                Some(x)
            } else {
                self.ordered_arbitrary(step, max_cplx)
            }
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        let x = self.mutator.random_arbitrary(max_cplx);
        if (self.filter)(&x.0) {
            x
        } else {
            self.random_arbitrary(max_cplx)
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if let Some((t, cplx)) = self.mutator.ordered_mutate(value, cache, step, max_cplx) {
            if (self.filter)(value) {
                Some((t, cplx))
            } else {
                self.mutator.unmutate(value, cache, t);
                self.ordered_mutate(value, cache, step, max_cplx)
            }
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let (t, cplx) = self.mutator.random_mutate(value, cache, max_cplx);
        if (self.filter)(value) {
            (t, cplx)
        } else {
            self.mutator.unmutate(value, cache, t);
            self.random_mutate(value, cache, max_cplx)
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value, cache, t)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn lens<'a>(&self, value: &'a T, cache: &'a Self::Cache, path: &Self::LensPath) -> &'a dyn std::any::Any {
        self.mutator.lens(value, cache, path)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(
        &self,
        value: &T,
        cache: &Self::Cache,
        register_path: &mut dyn FnMut(std::any::TypeId, Self::LensPath),
    ) {
        self.mutator.all_paths(value, cache, register_path)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn crossover_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        let (t, cplx) = self.mutator.crossover_mutate(value, cache, subvalue_provider, max_cplx);
        if (self.filter)(value) {
            (t, cplx)
        } else {
            self.mutator.unmutate(value, cache, t);
            self.crossover_mutate(value, cache, subvalue_provider, max_cplx)
        }
    }
}
