extern crate self as fuzzcheck;

use std::ops::{Bound, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

use fuzzcheck_mutators_derive::make_mutator;

use crate::mutators::map::MapMutator;
use crate::mutators::tuples::{Tuple2, Tuple2Mutator, TupleMutatorWrapper};
use crate::mutators::Wrapper;
use crate::{DefaultMutator, Mutator};

make_mutator! {
    name: RangeMutator,
    default: true,
    type: pub struct Range<Idx> {
        start: Idx,
        end: Idx,
    }
}

make_mutator! {
    name: RangeFromMutator,
    default: true,
    type: pub struct RangeFrom<Idx> {
        start: Idx,
    }
}

make_mutator! {
    name: RangeToMutator,
    default: true,
    type: pub struct RangeTo<Idx> {
        end: Idx,
    }
}

make_mutator! {
    name: RangeToInclusiveMutator,
    default: true,
    type: pub struct RangeToInclusive<Idx> {
        end: Idx,
    }
}

make_mutator! {
    name: RangeFullMutator,
    default: true,
    type: pub struct RangeFull;
}

make_mutator! {
    name: BoundMutator,
    default: true,
    type: pub enum Bound<T> {
        Included(T),
        Excluded(T),
        Unbounded,
    }
}

#[coverage(off)]
fn range_inclusive_from_tuple<T: Clone>(t: &(T, T)) -> RangeInclusive<T> {
    t.0.clone()..=t.1.clone()
}
#[coverage(off)]
fn tuple_from_range_inclusive<T: Clone>(r: &RangeInclusive<T>) -> Option<(T, T)> {
    Some((r.start().clone(), r.end().clone()))
}

#[coverage(off)]
fn range_cplx<T>(_r: &RangeInclusive<T>, orig_cplx: f64) -> f64 {
    orig_cplx
}

pub type RangeInclusiveMutator<T, M> = Wrapper<
    MapMutator<
        (T, T),
        RangeInclusive<T>,
        TupleMutatorWrapper<Tuple2Mutator<M, M>, Tuple2<T, T>>,
        fn(&RangeInclusive<T>) -> Option<(T, T)>,
        fn(&(T, T)) -> RangeInclusive<T>,
        fn(&RangeInclusive<T>, f64) -> f64,
    >,
>;

impl<T, M> RangeInclusiveMutator<T, M>
where
    T: Clone,
    M: Mutator<T> + Clone,
{
    #[coverage(off)]
    pub fn new(m: M) -> Self {
        Wrapper(MapMutator::new(
            TupleMutatorWrapper::new(Tuple2Mutator::new(m.clone(), m)),
            tuple_from_range_inclusive,
            range_inclusive_from_tuple,
            range_cplx,
        ))
    }
}
impl<T> DefaultMutator for RangeInclusive<T>
where
    T: 'static + Clone + DefaultMutator,
    T::Mutator: Clone,
{
    type Mutator = RangeInclusiveMutator<T, T::Mutator>;
    #[coverage(off)]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator())
    }
}
