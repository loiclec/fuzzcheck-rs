use std::rc::Rc;

use crate::DefaultMutator;
use crate::Mutator;

/// Default mutator of `Rc<T>`
#[derive(Default)]
pub struct RcMutator<M> {
    mutator: M,
}
impl<M> RcMutator<M> {
    #[no_coverage]
    pub fn new(mutator: M) -> Self {
        Self { mutator }
    }
}

impl<T: Clone, M: Mutator<T>> Mutator<Rc<T>> for RcMutator<M> {
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
    fn validate_value(&self, value: &Rc<T>) -> Option<Self::Cache> {
        self.mutator.validate_value(value)
    }
    fn default_mutation_step(&self, value: &Rc<T>, cache: &Self::Cache) -> Self::MutationStep {
        self.mutator.default_mutation_step(value.as_ref(), cache)
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
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        let mut v = value.as_ref().clone();
        let res = self.mutator.ordered_mutate(&mut v, cache, step, max_cplx);
        *value = Rc::new(v);
        res
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut Rc<T>, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let mut v = value.as_ref().clone();
        let res = self.mutator.random_mutate(&mut v, cache, max_cplx);
        *value = Rc::new(v);
        res
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut Rc<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        let mut v = value.as_ref().clone();
        self.mutator.unmutate(&mut v, cache, t);
        *value = Rc::new(v);
    }

    #[doc(hidden)]
    type RecursingPartIndex = M::RecursingPartIndex;
    #[doc(hidden)]
    #[no_coverage]
    fn default_recursing_part_index(&self, value: &Rc<T>, cache: &Self::Cache) -> Self::RecursingPartIndex {
        self.mutator.default_recursing_part_index(value, cache)
    }
    #[doc(hidden)]
    #[no_coverage]
    fn recursing_part<'a, V, N>(
        &self,
        parent: &N,
        value: &'a Rc<T>,
        index: &mut Self::RecursingPartIndex,
    ) -> Option<&'a V>
    where
        V: Clone + 'static,
        N: Mutator<V>,
    {
        self.mutator.recursing_part::<V, N>(parent, value, index)
    }
}

impl<T> DefaultMutator for Rc<T>
where
    T: DefaultMutator,
{
    #[doc(hidden)]
    type Mutator = RcMutator<<T as DefaultMutator>::Mutator>;
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator())
    }
}
