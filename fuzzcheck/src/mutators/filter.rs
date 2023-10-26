use std::any::Any;

use crate::Mutator;

/// A [`FilterMutator`] provides a way to filter values outputted by a mutator.
/// Given any [`Mutator<Value=T>`] and a function [`Fn(&T) -> bool`] it creates
/// a new mutator which can generate all the values of `T` the underlying
/// mutator can, except those for which the filtering function returns false.
pub struct FilterMutator<M, F> {
    mutator: M,
    filter: F,
}

impl<M, F> FilterMutator<M, F> {
    /// Creates a new [`FilterMutator`].
    ///
    /// Note that the mutator will filter all values for which the filtering
    /// function returns _false_.
    pub fn new<T>(mutator: M, filter: F) -> FilterMutator<M, F>
    where
        M: Mutator<T>,
        T: Clone + 'static,
        F: Fn(&T) -> bool,
        Self: 'static,
    {
        FilterMutator { mutator, filter }
    }
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
    fn is_valid(&self, value: &T) -> bool {
        self.mutator.is_valid(value) && (self.filter)(value)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        let x = self.mutator.validate_value(value);
        if x.is_some() && (self.filter)(value) == false {
            None
        } else {
            x
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        self.mutator.default_mutation_step(value, cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        self.mutator.global_search_space_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        loop {
            let x = self.mutator.ordered_arbitrary(step, max_cplx);
            if let Some(x) = x {
                if (self.filter)(&x.0) {
                    return Some(x);
                }
            } else {
                return None;
            }
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        loop {
            let x = self.mutator.random_arbitrary(max_cplx);
            if (self.filter)(&x.0) {
                return x;
            }
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        loop {
            if let Some((t, cplx)) = self
                .mutator
                .ordered_mutate(value, cache, step, subvalue_provider, max_cplx)
            {
                if (self.filter)(value) {
                    return Some((t, cplx));
                } else {
                    self.mutator.unmutate(value, cache, t);
                }
            } else {
                return None;
            }
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        loop {
            let (t, cplx) = self.mutator.random_mutate(value, cache, max_cplx);
            if (self.filter)(value) {
                return (t, cplx);
            } else {
                self.mutator.unmutate(value, cache, t);
            }
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value, cache, t)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, value: &'a T, cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        self.mutator.visit_subvalues(value, cache, visit)
    }
}
