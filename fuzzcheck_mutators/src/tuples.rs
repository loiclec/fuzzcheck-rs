use std::marker::PhantomData;

use fuzzcheck_traits::Mutator;

pub trait RefTypes {
    type Owned;
    type Ref<'a>: Copy;
    type Mut<'a>;
    fn get_ref_from_mut<'a>(v: &'a Self::Mut<'a>) -> Self::Ref<'a>;
}

pub struct Tuple1<T: 'static> {
    _phantom: PhantomData<T>,
}
impl<T: 'static> RefTypes for Tuple1<T> {
    type Owned = T;
    type Ref<'a> = &'a T;
    type Mut<'a> = &'a mut T;
    fn get_ref_from_mut<'a>(v: &'a Self::Mut<'a>) -> Self::Ref<'a> {
        v
    }
}
impl<T: 'static> TupleStructure<Tuple1<T>> for T {
    fn get_ref<'a>(&'a self) -> &'a T {
        self
    }

    fn get_mut<'a>(&'a mut self) -> &'a mut T {
        self
    }
    fn new(t: T) -> Self {
        t
    }
}

pub struct Tuple1Mutator<T, M>
where
    T: ::std::clone::Clone,
    M: ::fuzzcheck_traits::Mutator<T>,
{
    pub mutator: M,
    _phantom: ::std::marker::PhantomData<(T, T)>,
}
impl<T, M> Tuple1Mutator<T, M>
where
    T: ::std::clone::Clone,
    M: ::fuzzcheck_traits::Mutator<T>,
{
    pub fn new(mutator: M) -> Self {
        Self {
            mutator,
            _phantom: PhantomData,
        }
    }
}

impl<T, M> Default for Tuple1Mutator<T, M>
where
    T: ::std::clone::Clone,
    M: ::fuzzcheck_traits::Mutator<T>,
    M: Default,
{
    fn default() -> Self {
        Self {
            mutator: <_>::default(),
            _phantom: PhantomData,
        }
    }
}

impl<T, M> TupleMutator<T, Tuple1<T>> for Tuple1Mutator<T, M>
where
    T: ::std::clone::Clone + 'static,
    M: ::fuzzcheck_traits::Mutator<T>,
{
    type Cache = M::Cache;
    type MutationStep = M::MutationStep;
    type ArbitraryStep = M::ArbitraryStep;
    type UnmutateToken = M::UnmutateToken;

    fn complexity<'a>(&'a self, value: &'a T, cache: &'a Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    fn cache_from_value<'a>(&'a self, value: &'a T) -> Self::Cache {
        self.mutator.cache_from_value(value)
    }

    fn initial_step_from_value<'a>(&'a self, value: &'a T) -> Self::MutationStep {
        self.mutator.initial_step_from_value(value)
    }

    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, Self::Cache)> {
        self.mutator.ordered_arbitrary(step, max_cplx)
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (T, Self::Cache) {
        self.mutator.random_arbitrary(max_cplx)
    }

    fn ordered_mutate<'a>(
        &'a self,
        value: &'a mut T,
        cache: &'a mut Self::Cache,
        step: &'a mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        self.mutator.ordered_mutate(value, cache, step, max_cplx)
    }

    fn random_mutate<'a>(&'a self, value: &'a mut T, cache: &'a mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        self.mutator.random_mutate(value, cache, max_cplx)
    }

    fn unmutate<'a>(&'a self, value: &'a mut T, cache: &'a mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value, cache, t)
    }
}

pub struct Wrapped<T: 'static> {
    _phantom: PhantomData<T>,
}
impl<T: 'static> RefTypes for Wrapped<T> {
    type Owned = T;
    type Ref<'a> = &'a T;
    type Mut<'a> = &'a mut T;

    fn get_ref_from_mut<'a>(v: &'a Self::Mut<'a>) -> Self::Ref<'a> {
        v
    }
}

impl<T, U, M> TupleMutator<U, Wrapped<T>> for Tuple1Mutator<T, M>
where
    U: TupleStructure<Wrapped<T>>,
    T: ::std::clone::Clone + 'static,
    M: ::fuzzcheck_traits::Mutator<T>,
{
    type Cache = M::Cache;
    type MutationStep = M::MutationStep;
    type ArbitraryStep = M::ArbitraryStep;
    type UnmutateToken = M::UnmutateToken;

    fn complexity<'a>(&'a self, value: &'a T, cache: &'a Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    fn cache_from_value<'a>(&'a self, value: &'a T) -> Self::Cache {
        self.mutator.cache_from_value(value)
    }

    fn initial_step_from_value<'a>(&'a self, value: &'a T) -> Self::MutationStep {
        self.mutator.initial_step_from_value(value)
    }

    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(U, Self::Cache)> {
        if let Some((value, cache)) = self.mutator.ordered_arbitrary(step, max_cplx) {
            Some((U::new(value), cache))
        } else {
            None
        }
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (U, Self::Cache) {
        let (value, cache) = self.mutator.random_arbitrary(max_cplx);
        (U::new(value), cache)
    }

    fn ordered_mutate<'a>(
        &'a self,
        value: &'a mut T,
        cache: &'a mut Self::Cache,
        step: &'a mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        self.mutator.ordered_mutate(value, cache, step, max_cplx)
    }

    fn random_mutate<'a>(&'a self, value: &'a mut T, cache: &'a mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        self.mutator.random_mutate(value, cache, max_cplx)
    }

    fn unmutate<'a>(&'a self, value: &'a mut T, cache: &'a mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value, cache, t)
    }
}

pub trait TupleStructure<TupleKind: RefTypes> {
    fn get_ref<'a>(&'a self) -> TupleKind::Ref<'a>;
    fn get_mut<'a>(&'a mut self) -> TupleKind::Mut<'a>;
    fn new(t: TupleKind::Owned) -> Self;
}

pub trait TupleMutator<T, TupleKind>
where
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
{
    type Cache: Clone;
    type MutationStep: Clone;
    type ArbitraryStep: Clone + Default;
    type UnmutateToken;

    fn complexity<'a>(&'a self, value: TupleKind::Ref<'a>, cache: &'a Self::Cache) -> f64;

    fn cache_from_value<'a>(&'a self, value: TupleKind::Ref<'a>) -> Self::Cache;

    fn initial_step_from_value<'a>(&'a self, value: TupleKind::Ref<'a>) -> Self::MutationStep;

    fn max_complexity(&self) -> f64;

    fn min_complexity(&self) -> f64;

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, Self::Cache)>;

    fn random_arbitrary(&self, max_cplx: f64) -> (T, Self::Cache);

    fn ordered_mutate<'a>(
        &'a self,
        value: TupleKind::Mut<'a>,
        cache: &'a mut Self::Cache,
        step: &'a mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken>;

    fn random_mutate<'a>(
        &'a self,
        value: TupleKind::Mut<'a>,
        cache: &'a mut Self::Cache,
        max_cplx: f64,
    ) -> Self::UnmutateToken;

    fn unmutate<'a>(&'a self, value: TupleKind::Mut<'a>, cache: &'a mut Self::Cache, t: Self::UnmutateToken);
}

pub struct TupleMutatorWrapper<T, M, TupleKind>
where
    T: Clone + 'static,
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
    M: TupleMutator<T, TupleKind>,
{
    pub mutator: M,
    _phantom: PhantomData<(T, TupleKind)>,
}
impl<T, M, TupleKind> TupleMutatorWrapper<T, M, TupleKind>
where
    T: Clone + 'static,
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
    M: TupleMutator<T, TupleKind>,
{
    pub fn new(mutator: M) -> Self {
        Self {
            mutator,
            _phantom: PhantomData,
        }
    }
}
impl<T, M, TupleKind> Default for TupleMutatorWrapper<T, M, TupleKind>
where
    T: Clone + 'static,
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
    M: TupleMutator<T, TupleKind>,
    M: Default,
{
    fn default() -> Self {
        Self {
            mutator: <_>::default(),
            _phantom: PhantomData,
        }
    }
}

impl<T, M, TupleKind> Mutator<T> for TupleMutatorWrapper<T, M, TupleKind>
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

    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value.get_ref(), cache)
    }

    fn cache_from_value(&self, value: &T) -> Self::Cache {
        self.mutator.cache_from_value(value.get_ref())
    }

    fn initial_step_from_value(&self, value: &T) -> Self::MutationStep {
        self.mutator.initial_step_from_value(value.get_ref())
    }

    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, Self::Cache)> {
        self.mutator.ordered_arbitrary(step, max_cplx)
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (T, Self::Cache) {
        self.mutator.random_arbitrary(max_cplx)
    }

    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        self.mutator.ordered_mutate(value.get_mut(), cache, step, max_cplx)
    }

    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        self.mutator.random_mutate(value.get_mut(), cache, max_cplx)
    }

    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value.get_mut(), cache, t)
    }
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
