extern crate self as fuzzcheck_mutators;

use crate::fuzzcheck_traits::Mutator;
use crate::map::MapMutator;
use crate::tuples::TupleMutatorWrapper;
use crate::tuples::{Tuple2, Tuple2Mutator};
use crate::wrapper::Wrapper;
use crate::DefaultMutator;
use fuzzcheck_mutators_derive::make_mutator;
use std::ops::{Bound, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

// TODO: RangeInclusiveMutator with a MapMutator

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

#[no_coverage]
fn range_inclusive_from_tuple<T: Clone>(t: &(T, T)) -> RangeInclusive<T> {
    t.0.clone()..=t.1.clone()
}
#[no_coverage]
fn tuple_from_range_inclusive<T: Clone>(r: &RangeInclusive<T>) -> Option<(T, T)> {
    Some((r.start().clone(), r.end().clone()))
}

pub type RangeInclusiveMutator<T, M> = Wrapper<
    MapMutator<
        (T, T),
        RangeInclusive<T>,
        TupleMutatorWrapper<Tuple2Mutator<M, M>, Tuple2<T, T>>,
        fn(&RangeInclusive<T>) -> Option<(T, T)>,
        fn(&(T, T)) -> RangeInclusive<T>,
    >,
>;

impl<T, M> RangeInclusiveMutator<T, M>
where
    T: Clone,
    M: Mutator<T> + Clone,
{
    #[no_coverage]
    pub fn new(m: M) -> Self {
        Wrapper(MapMutator::new(
            TupleMutatorWrapper::new(Tuple2Mutator::new(m.clone(), m)),
            tuple_from_range_inclusive,
            range_inclusive_from_tuple,
        ))
    }
}
impl<T> DefaultMutator for RangeInclusive<T>
where
    T: 'static + Clone + DefaultMutator,
    T::Mutator: Clone,
{
    type Mutator = RangeInclusiveMutator<T, T::Mutator>;
    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator())
    }
}
