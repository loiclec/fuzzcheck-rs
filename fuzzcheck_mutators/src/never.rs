use fuzzcheck_traits::Mutator;

use crate::{RefTypes, TupleMutator, TupleStructure};

pub enum NeverMutator {}

impl<T: Clone> Mutator<T> for NeverMutator {
    type Cache = ();
    type MutationStep = ();
    type ArbitraryStep = ();
    type UnmutateToken = ();
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        unreachable!()
    }

    fn validate_value(&self, value: &T) -> Option<(Self::Cache, Self::MutationStep)> {
        unreachable!()
    }

    fn max_complexity(&self) -> f64 {
        unreachable!()
    }

    fn min_complexity(&self) -> f64 {
        unreachable!()
    }

    fn complexity(&self, _value: &T, _cache: &Self::Cache) -> f64 {
        unreachable!()
    }

    fn ordered_arbitrary(
        &self,
        _step: &mut Self::ArbitraryStep,
        _max_cplx: f64,
    ) -> Option<(T, Self::Cache, Self::MutationStep)> {
        unreachable!()
    }

    fn random_arbitrary(&self, _max_cplx: f64) -> (T, Self::Cache, Self::MutationStep) {
        unreachable!()
    }

    fn ordered_mutate(
        &self,
        _value: &mut T,
        _cache: &mut Self::Cache,
        _step: &mut Self::MutationStep,
        _max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        unreachable!()
    }

    fn random_mutate(&self, _value: &mut T, _cache: &mut Self::Cache, _max_cplx: f64) -> Self::UnmutateToken {
        unreachable!()
    }

    fn unmutate(&self, _value: &mut T, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {
        unreachable!()
    }
}

impl<T: Clone, TupleKind: RefTypes> TupleMutator<T, TupleKind> for NeverMutator
where
    T: TupleStructure<TupleKind>,
{
    type Cache = ();
    type MutationStep = ();
    type ArbitraryStep = ();
    type UnmutateToken = ();

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        unreachable!()
    }

    fn complexity<'a>(&'a self, _value: TupleKind::Ref<'a>, _cache: &'a Self::Cache) -> f64 {
        unreachable!()
    }

    fn validate_value<'a>(&'a self, value: TupleKind::Ref<'a>) -> Option<(Self::Cache, Self::MutationStep)> {
        unreachable!()
    }

    fn max_complexity(&self) -> f64 {
        unreachable!()
    }

    fn min_complexity(&self) -> f64 {
        unreachable!()
    }

    fn ordered_arbitrary(
        &self,
        _step: &mut Self::ArbitraryStep,
        _max_cplx: f64,
    ) -> Option<(T, Self::Cache, Self::MutationStep)> {
        unreachable!()
    }

    fn random_arbitrary(&self, _max_cplx: f64) -> (T, Self::Cache, Self::MutationStep) {
        unreachable!()
    }

    fn ordered_mutate<'a>(
        &'a self,
        _value: TupleKind::Mut<'a>,
        _cache: &'a mut Self::Cache,
        _step: &'a mut Self::MutationStep,
        _max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        unreachable!()
    }

    fn random_mutate<'a>(
        &'a self,
        _value: TupleKind::Mut<'a>,
        _cache: &'a mut Self::Cache,
        _max_cplx: f64,
    ) -> Self::UnmutateToken {
        unreachable!()
    }

    fn unmutate<'a>(&'a self, _value: TupleKind::Mut<'a>, _cache: &'a mut Self::Cache, _t: Self::UnmutateToken) {
        unreachable!()
    }
}
