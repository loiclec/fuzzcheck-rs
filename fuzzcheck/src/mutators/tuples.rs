use std::marker::PhantomData;

// use fuzzcheck_traits;
use crate::Mutator;

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

pub trait TupleMutator<T, TupleKind>: Sized + 'static
where
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
{
    type Cache: Clone;
    type MutationStep: Clone;
    type ArbitraryStep: Clone;
    type UnmutateToken;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep;

    fn complexity<'a>(&self, value: TupleKind::Ref<'a>, cache: &'a Self::Cache) -> f64;

    fn validate_value<'a>(&self, value: TupleKind::Ref<'a>) -> Option<(Self::Cache, Self::MutationStep)>;

    fn max_complexity(&self) -> f64;

    fn min_complexity(&self) -> f64;

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)>;

    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64);

    fn ordered_mutate<'a>(
        &self,
        value: TupleKind::Mut<'a>,
        cache: &'a mut Self::Cache,
        step: &'a mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)>;

    fn random_mutate<'a>(
        &self,
        value: TupleKind::Mut<'a>,
        cache: &'a mut Self::Cache,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64);

    fn unmutate<'a>(&self, value: TupleKind::Mut<'a>, cache: &'a mut Self::Cache, t: Self::UnmutateToken);

    type RecursingPartIndex: Clone;
    fn default_recursing_part_index<'a>(
        &self,
        value: TupleKind::Ref<'a>,
        cache: &Self::Cache,
    ) -> Self::RecursingPartIndex;
    fn recursing_part<'a, V, N>(
        &self,
        parent: &N,
        value: TupleKind::Ref<'a>,
        index: &mut Self::RecursingPartIndex,
    ) -> Option<&'a V>
    where
        V: Clone + 'static,
        N: Mutator<V>;
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
    TupleKind: RefTypes + 'static,
    T: TupleStructure<TupleKind>,
    M: TupleMutator<T, TupleKind>,
{
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
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value.get_ref(), cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<(Self::Cache, Self::MutationStep)> {
        self.mutator.validate_value(value.get_ref())
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
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        self.mutator.ordered_arbitrary(step, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        self.mutator.random_arbitrary(max_cplx)
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
        self.mutator.ordered_mutate(value.get_mut(), cache, step, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        self.mutator.random_mutate(value.get_mut(), cache, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value.get_mut(), cache, t)
    }

    #[doc(hidden)]
    type RecursingPartIndex = M::RecursingPartIndex;

    #[doc(hidden)]
    #[no_coverage]
    fn default_recursing_part_index(&self, value: &T, cache: &Self::Cache) -> Self::RecursingPartIndex {
        self.mutator.default_recursing_part_index(value.get_ref(), cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn recursing_part<'a, V, N>(&self, parent: &N, value: &'a T, index: &mut Self::RecursingPartIndex) -> Option<&'a V>
    where
        V: Clone + 'static,
        N: Mutator<V>,
    {
        self.mutator.recursing_part::<V, N>(parent, value.get_ref(), index)
    }
}

pub use tuple0::{Tuple0, Tuple0Mutator};
mod tuple0 {
    use super::TupleMutator;
    use crate::mutators::tuples::RefTypes;
    use crate::mutators::tuples::TupleStructure;
    use crate::Mutator;

    pub struct Tuple0;
    impl RefTypes for Tuple0 {
        type Owned = ();
        type Ref<'a> = ();
        type Mut<'a> = ();
        fn get_ref_from_mut<'a>(_v: &'a Self::Mut<'a>) -> Self::Ref<'a> {
            ()
        }
    }
    impl TupleStructure<Tuple0> for () {
        fn get_ref(&self) -> <Tuple0 as RefTypes>::Ref<'_> {
            ()
        }

        fn get_mut(&mut self) -> <Tuple0 as RefTypes>::Mut<'_> {
            ()
        }

        fn new(_t: <Tuple0 as RefTypes>::Owned) -> Self {
            ()
        }
    }
    pub struct Tuple0Mutator;
    impl TupleMutator<(), Tuple0> for Tuple0Mutator {
        #[doc(hidden)]
        type Cache = ();
        #[doc(hidden)]
        type MutationStep = bool;
        #[doc(hidden)]
        type ArbitraryStep = bool;
        #[doc(hidden)]
        type UnmutateToken = ();

        #[doc(hidden)]
        #[no_coverage]
        fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
            false
        }

        #[doc(hidden)]
        #[no_coverage]
        fn complexity(&self, _value: (), _cache: &Self::Cache) -> f64 {
            0.0
        }

        #[doc(hidden)]
        #[no_coverage]
        fn validate_value(&self, _value: ()) -> Option<(Self::Cache, Self::MutationStep)> {
            Some(((), false))
        }

        #[doc(hidden)]
        #[no_coverage]
        fn max_complexity(&self) -> f64 {
            0.0
        }

        #[doc(hidden)]
        #[no_coverage]
        fn min_complexity(&self) -> f64 {
            0.0
        }

        #[doc(hidden)]
        #[no_coverage]
        fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, _max_cplx: f64) -> Option<((), f64)> {
            if !*step {
                *step = true;
                Some(((), 0.0))
            } else {
                None
            }
        }

        #[doc(hidden)]
        #[no_coverage]
        fn random_arbitrary(&self, _max_cplx: f64) -> ((), f64) {
            ((), 0.0)
        }

        #[doc(hidden)]
        #[no_coverage]
        fn ordered_mutate(
            &self,
            _value: (),
            _cache: &mut Self::Cache,
            step: &mut Self::MutationStep,
            _max_cplx: f64,
        ) -> Option<(Self::UnmutateToken, f64)> {
            if !*step {
                *step = true;
                Some(((), 0.0))
            } else {
                None
            }
        }

        #[doc(hidden)]
        #[no_coverage]
        fn random_mutate(&self, _value: (), _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
            ((), 0.0)
        }

        #[doc(hidden)]
        #[no_coverage]
        fn unmutate(&self, _value: (), _cache: &mut Self::Cache, _t: Self::UnmutateToken) {}

        #[doc(hidden)]
        type RecursingPartIndex = ();

        #[doc(hidden)]
        #[no_coverage]
        fn default_recursing_part_index(&self, _value: (), _cache: &Self::Cache) -> Self::RecursingPartIndex {}

        #[doc(hidden)]
        #[no_coverage]
        fn recursing_part<'a, V, N>(
            &self,
            _parent: &N,
            _value: (),
            _index: &mut Self::RecursingPartIndex,
        ) -> Option<&'a V>
        where
            V: Clone + 'static,
            N: Mutator<V>,
        {
            None
        }
    }
}

pub use tuple1::{Tuple1, Tuple1Mutator};
mod tuple1 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(1);
}
pub use tuple2::{Tuple2, Tuple2Mutator};
mod tuple2 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(2);
}
pub use tuple3::{Tuple3, Tuple3Mutator};
mod tuple3 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(3);
}
pub use tuple4::{Tuple4, Tuple4Mutator};
mod tuple4 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(4);
}
pub use tuple5::{Tuple5, Tuple5Mutator};
mod tuple5 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(5);
}
pub use tuple6::{Tuple6, Tuple6Mutator};
mod tuple6 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(6);
}
pub use tuple7::{Tuple7, Tuple7Mutator};
mod tuple7 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(7);
}
pub use tuple8::{Tuple8, Tuple8Mutator};
mod tuple8 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(8);
}
pub use tuple9::{Tuple9, Tuple9Mutator};
mod tuple9 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(9);
}
pub use tuple10::{Tuple10, Tuple10Mutator};
mod tuple10 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(10);
}
