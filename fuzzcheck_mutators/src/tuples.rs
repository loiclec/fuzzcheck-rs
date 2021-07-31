use std::marker::PhantomData;

// use crate::fuzzcheck_traits;
use crate::fuzzcheck_traits::Mutator;

pub trait RefTypes {
    type Owned;
    type Ref<'a>: Copy;
    type Mut<'a>;
    fn get_ref_from_mut<'a>(v: &'a Self::Mut<'a>) -> Self::Ref<'a>;
}

pub trait TupleStructure<TupleKind: RefTypes> {
    fn get_ref(&self) -> TupleKind::Ref<'_>;
    fn get_mut(&mut self) -> TupleKind::Mut<'_>;
    fn new(t: TupleKind::Owned) -> Self;
}

pub trait TupleMutator<T, TupleKind>: Sized
where
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
{
    type Cache;
    type MutationStep;
    type ArbitraryStep;
    type UnmutateToken;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep;

    fn complexity<'a>(&'a self, value: TupleKind::Ref<'a>, cache: &'a Self::Cache) -> f64;

    fn validate_value<'a>(&'a self, value: TupleKind::Ref<'a>) -> Option<(Self::Cache, Self::MutationStep)>;

    fn max_complexity(&self) -> f64;

    fn min_complexity(&self) -> f64;

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)>;

    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64);

    fn ordered_mutate<'a>(
        &'a self,
        value: TupleKind::Mut<'a>,
        cache: &'a mut Self::Cache,
        step: &'a mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)>;

    fn random_mutate<'a>(
        &'a self,
        value: TupleKind::Mut<'a>,
        cache: &'a mut Self::Cache,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64);

    fn unmutate<'a>(&'a self, value: TupleKind::Mut<'a>, cache: &'a mut Self::Cache, t: Self::UnmutateToken);
}

pub struct TupleMutatorWrapper<M, TupleKind>
where
    TupleKind: RefTypes,
{
    pub mutator: M,
    _phantom: PhantomData<TupleKind>,
}
impl<M, TupleKind> TupleMutatorWrapper<M, TupleKind>
where
    TupleKind: RefTypes,
{
    #[no_coverage]
    pub fn new(mutator: M) -> Self {
        Self {
            mutator,
            _phantom: PhantomData,
        }
    }
}
impl<M, TupleKind> Default for TupleMutatorWrapper<M, TupleKind>
where
    TupleKind: RefTypes,
    M: Default,
{
    #[no_coverage]
    fn default() -> Self {
        Self {
            mutator: <_>::default(),
            _phantom: PhantomData,
        }
    }
}

impl<T, M, TupleKind> Mutator<T> for TupleMutatorWrapper<M, TupleKind>
where
    T: Clone + 'static,
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
    M: TupleMutator<T, TupleKind>,
{
    type Cache = M::Cache;
    type MutationStep = M::MutationStep;
    type ArbitraryStep = M::ArbitraryStep;
    type UnmutateToken = M::UnmutateToken;

    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value.get_ref(), cache)
    }

    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<(Self::Cache, Self::MutationStep)> {
        self.mutator.validate_value(value.get_ref())
    }

    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        self.mutator.ordered_arbitrary(step, max_cplx)
    }

    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        self.mutator.random_arbitrary(max_cplx)
    }

    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        self.mutator.ordered_mutate(value.get_mut(), cache, step, max_cplx)
    }

    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        self.mutator.random_mutate(value.get_mut(), cache, max_cplx)
    }

    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value.get_mut(), cache, t)
    }
}

pub use tuple1::{Tuple1, Tuple1Mutator};
mod tuple1 {
    #![allow(clippy::needless_update)]
    extern crate self as fuzzcheck_mutators;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(1);
}
pub use tuple2::{Tuple2, Tuple2Mutator};
mod tuple2 {
    extern crate self as fuzzcheck_mutators;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(2);
}
pub use tuple3::{Tuple3, Tuple3Mutator};
mod tuple3 {
    extern crate self as fuzzcheck_mutators;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(3);
}
pub use tuple4::{Tuple4, Tuple4Mutator};
mod tuple4 {
    extern crate self as fuzzcheck_mutators;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(4);
}
pub use tuple5::{Tuple5, Tuple5Mutator};
mod tuple5 {
    extern crate self as fuzzcheck_mutators;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(5);
}
pub use tuple6::{Tuple6, Tuple6Mutator};
mod tuple6 {
    extern crate self as fuzzcheck_mutators;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(6);
}
pub use tuple7::{Tuple7, Tuple7Mutator};
mod tuple7 {
    extern crate self as fuzzcheck_mutators;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(7);
}
pub use tuple8::{Tuple8, Tuple8Mutator};
mod tuple8 {
    extern crate self as fuzzcheck_mutators;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(8);
}
pub use tuple9::{Tuple9, Tuple9Mutator};
mod tuple9 {
    extern crate self as fuzzcheck_mutators;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(9);
}
pub use tuple10::{Tuple10, Tuple10Mutator};
mod tuple10 {
    extern crate self as fuzzcheck_mutators;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(10);
}
