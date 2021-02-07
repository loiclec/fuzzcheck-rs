use fuzzcheck_traits::Mutator;

use crate::{DefaultMutator, Either2, Enum1PayloadMutator, Enum1PayloadStructure, Tuple1, Tuple1Mutator};

// TODO: use proc_macro for that

impl<T> Enum1PayloadStructure for Option<T>
where
    T: 'static,
{
    type T0 = T;
    type TupleKind0 = Tuple1<T>;

    fn get_ref<'a>(&'a self) -> Either2<&'a T, usize> {
        match self {
            Some(x) => Either2::T0(x),
            None => Either2::T1(0),
        }
    }
    fn get_mut<'a>(&'a mut self) -> Either2<&'a mut T, usize> {
        match self {
            Some(x) => Either2::T0(x),
            None => Either2::T1(0),
        }
    }
    fn new(t: Either2<Self::T0, usize>) -> Self {
        match t {
            Either2::T0(x) => Some(x),
            Either2::T1(_) => None,
        }
    }
}

pub struct OptionMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    pub mutator: Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>>,
}
impl<T, M> OptionMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    pub fn new(mutator: M) -> Self {
        Self {
            mutator: Enum1PayloadMutator::new(Tuple1Mutator::new(mutator)),
        }
    }
}
impl<T, M> Default for OptionMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
    Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>>: Default,
{
    fn default() -> Self {
        Self {
            mutator: <_>::default(),
        }
    }
}
impl<T, M> Mutator<Option<T>> for OptionMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type Cache = <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::Cache;
    type MutationStep =
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::MutationStep;
    type ArbitraryStep =
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::ArbitraryStep;
    type UnmutateToken =
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::UnmutateToken;

    fn cache_from_value(&self, value: &Option<T>) -> Self::Cache {
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::cache_from_value(
            &self.mutator,
            value,
        )
    }

    fn initial_step_from_value(&self, value: &Option<T>) -> Self::MutationStep {
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::initial_step_from_value(
            &self.mutator,
            value,
        )
    }

    fn max_complexity(&self) -> f64 {
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::max_complexity(
            &self.mutator,
        )
    }

    fn min_complexity(&self) -> f64 {
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::min_complexity(
            &self.mutator,
        )
    }

    fn complexity(&self, value: &Option<T>, cache: &Self::Cache) -> f64 {
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::complexity(
            &self.mutator,
            value,
            cache,
        )
    }

    fn ordered_arbitrary(&mut self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(Option<T>, Self::Cache)> {
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::ordered_arbitrary(
            &mut self.mutator,
            step,
            max_cplx,
        )
    }

    fn random_arbitrary(&mut self, max_cplx: f64) -> (Option<T>, Self::Cache) {
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::random_arbitrary(
            &mut self.mutator,
            max_cplx,
        )
    }

    fn ordered_mutate(
        &mut self,
        value: &mut Option<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::ordered_mutate(
            &mut self.mutator,
            value,
            cache,
            step,
            max_cplx,
        )
    }

    fn random_mutate(&mut self, value: &mut Option<T>, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::random_mutate(
            &mut self.mutator,
            value,
            cache,
            max_cplx,
        )
    }

    fn unmutate(&self, value: &mut Option<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        <Enum1PayloadMutator<T, Tuple1Mutator<T, M>, crate::Tuple1<T>> as Mutator<Option<T>>>::unmutate(
            &self.mutator,
            value,
            cache,
            t,
        )
    }
}

impl<T> DefaultMutator for Option<T>
where
    T: DefaultMutator + 'static,
{
    type Mutator = OptionMutator<T, <T as DefaultMutator>::Mutator>;

    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator())
    }
}
