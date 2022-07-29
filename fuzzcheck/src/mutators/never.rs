use std::any::Any;

use crate::mutators::tuples::{RefTypes, TupleMutator, TupleStructure};
use crate::Mutator;

pub enum NeverMutator {}

impl<T: Clone + 'static> Mutator<T> for NeverMutator {
    #[doc(hidden)]
    type Cache = ();
    #[doc(hidden)]
    type MutationStep = ();
    #[doc(hidden)]
    type ArbitraryStep = ();
    #[doc(hidden)]
    type UnmutateToken = ();

    #[doc(hidden)]
    #[no_coverage]
    fn initialize(&self) {}

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn is_valid(&self, _value: &T) -> bool {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, _value: &T) -> Option<Self::Cache> {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, _value: &T, _cache: &Self::Cache) -> Self::MutationStep {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn global_search_space_complexity(&self) -> f64 {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, _value: &T, _cache: &Self::Cache) -> f64 {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, _step: &mut Self::ArbitraryStep, _max_cplx: f64) -> Option<(T, f64)> {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, _max_cplx: f64) -> (T, f64) {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        _value: &mut T,
        _cache: &mut Self::Cache,
        _step: &mut Self::MutationStep,
        _subvalue_provider: &dyn crate::SubValueProvider,
        _max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, _value: &mut T, _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, _value: &mut T, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn visit_subvalues<'a>(&self, _value: &'a T, _cache: &'a Self::Cache, _visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        unreachable!()
    }
}

impl<T: Clone + 'static, TupleKind: RefTypes> TupleMutator<T, TupleKind> for NeverMutator
where
    T: TupleStructure<TupleKind>,
{
    #[doc(hidden)]
    type Cache = ();
    #[doc(hidden)]
    type MutationStep = ();
    #[doc(hidden)]
    type ArbitraryStep = ();
    #[doc(hidden)]
    type UnmutateToken = ();

    #[doc(hidden)]
    #[no_coverage]
    fn initialize(&self) {}

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity<'a>(&self, _value: TupleKind::Ref<'a>, _cache: &'a Self::Cache) -> f64 {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn is_valid(&self, _value: TupleKind::Ref<'_>) -> bool {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, _value: TupleKind::Ref<'_>) -> Option<Self::Cache> {
        unreachable!()
    }
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step<'a>(&self, _value: TupleKind::Ref<'a>, _cache: &'a Self::Cache) -> Self::MutationStep {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn global_search_space_complexity(&self) -> f64 {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, _step: &mut Self::ArbitraryStep, _max_cplx: f64) -> Option<(T, f64)> {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, _max_cplx: f64) -> (T, f64) {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate<'a>(
        &self,
        _value: TupleKind::Mut<'a>,
        _cache: &'a mut Self::Cache,
        _step: &'a mut Self::MutationStep,
        _subvalue_provider: &dyn crate::SubValueProvider,
        _max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate<'a>(
        &self,
        _value: TupleKind::Mut<'a>,
        _cache: &'a mut Self::Cache,
        _max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate<'a>(&self, _value: TupleKind::Mut<'a>, _cache: &'a mut Self::Cache, _t: Self::UnmutateToken) {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn visit_subvalues<'a>(
        &self,
        _value: TupleKind::Ref<'a>,
        _cache: &'a Self::Cache,
        _visit: &mut dyn FnMut(&'a dyn Any, f64),
    ) {
        unreachable!()
    }
}
