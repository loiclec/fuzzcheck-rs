use std::any::TypeId;
use std::ops::Range;

use super::insert_many_elements::insert_many;
use super::VecMutator;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::{Mutator, SubValueProvider};

pub struct CrossoverInsertSlice;

#[derive(Clone)]
pub struct CrossoverInsertSliceStep;

pub enum ConcreteCrossoverInsertSlice<T> {
    Random(usize),
    InsertSlice { idx: usize, slice: Vec<T>, added_cplx: f64 },
}
pub enum RevertCrossoverInsertSlice<UT> {
    Random(UT, usize),
    InsertSlice { idxs: Range<usize> },
}

impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertCrossoverInsertSlice<M::UnmutateToken>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[coverage(off)]
    fn revert(
        self,
        mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) {
        match self {
            RevertCrossoverInsertSlice::Random(token, idx) => {
                mutator.m.unmutate(&mut value[idx], &mut cache.inner[idx], token);
            }
            RevertCrossoverInsertSlice::InsertSlice { idxs } => {
                let _ = value.drain(idxs);
            }
        }
    }
}

impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for CrossoverInsertSlice
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = !;
    type Step = CrossoverInsertSliceStep;
    type Concrete<'a> = ConcreteCrossoverInsertSlice<T>;
    type Revert = RevertCrossoverInsertSlice<M::UnmutateToken>;

    #[coverage(off)]
    fn default_random_step(&self, _mutator: &VecMutator<T, M>, _value: &Vec<T>) -> Option<Self::RandomStep> {
        None
    }

    #[coverage(off)]
    fn random<'a>(
        _mutator: &VecMutator<T, M>,
        _value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _random_step: &Self::RandomStep,
        _max_cplx: f64,
    ) -> Self::Concrete<'a> {
        unreachable!()
    }

    #[coverage(off)]
    fn default_step(
        &self,
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) -> Option<Self::Step> {
        if mutator.m.global_search_space_complexity() == 0. || *mutator.len_range.end() == value.len() {
            return None;
        }
        if value.is_empty() {
            None
        } else {
            Some(CrossoverInsertSliceStep)
        }
    }

    #[coverage(off)]
    fn from_step<'a>(
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _step: &'a mut Self::Step,
        subvalue_provider: &dyn SubValueProvider,
        max_cplx: f64,
    ) -> Option<Self::Concrete<'a>> {
        let value_cplx = mutator.complexity(value, cache);
        let spare_cplx = max_cplx - value_cplx;
        let choice = mutator.rng.usize(..value.len());
        if let Some((slice, _)) = subvalue_provider.get_random_subvalue(TypeId::of::<Vec<T>>(), f64::INFINITY) {
            let slice = slice.downcast_ref::<Vec<T>>().unwrap();
            if slice.is_empty() {
                Some(ConcreteCrossoverInsertSlice::Random(choice))
            } else {
                let start_insertion_idx = mutator.rng.usize(..=value.len());
                let max_added_len = std::cmp::min(slice.len(), *mutator.len_range.end() - value.len());
                assert_ne!(max_added_len, 0);
                let start_copied_slice = mutator.rng.usize(..slice.len());
                let (copied_slice, added_cplx) = {
                    let mut copied_slice = vec![];
                    let mut added_len = 0;
                    let mut added_cplx = 0.0;
                    let mut slice_idx = start_copied_slice;
                    // TODO: take into consideration the inherent complexity of the vector due to its length
                    while slice_idx < slice.len() && added_len < max_added_len {
                        let el = &slice[slice_idx];
                        slice_idx += 1;
                        if let Some(el_cache) = mutator.m.validate_value(el) {
                            let el_cplx = mutator.m.complexity(el, &el_cache);
                            if added_cplx + el_cplx < spare_cplx {
                                copied_slice.push(el.clone());
                                added_cplx += el_cplx;
                                added_len += 1;
                            } else {
                                continue;
                            }
                        }
                    }
                    (copied_slice, added_cplx)
                };
                Some(ConcreteCrossoverInsertSlice::InsertSlice {
                    idx: start_insertion_idx,
                    slice: copied_slice,
                    added_cplx,
                })
            }
        } else {
            Some(ConcreteCrossoverInsertSlice::Random(choice))
        }
    }

    #[coverage(off)]
    fn apply<'a>(
        mutation: Self::Concrete<'a>,
        mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _subvalue_provider: &dyn SubValueProvider,
        max_cplx: f64,
    ) -> (Self::Revert, f64) {
        match mutation {
            ConcreteCrossoverInsertSlice::Random(idx) => {
                let old_cplx = mutator.complexity(value, cache);
                let old_el_cplx = mutator.m.complexity(&value[idx], &cache.inner[idx]);
                let spare_cplx = max_cplx - (old_cplx - old_el_cplx);
                let (token, new_el_cplx) = mutator
                    .m
                    .random_mutate(&mut value[idx], &mut cache.inner[idx], spare_cplx);
                (
                    RevertCrossoverInsertSlice::Random(token, idx),
                    mutator.complexity_from_inner(cache.sum_cplx - old_el_cplx + new_el_cplx, value.len()),
                )
            }
            ConcreteCrossoverInsertSlice::InsertSlice { slice, idx, added_cplx } => {
                let r = RevertCrossoverInsertSlice::InsertSlice {
                    idxs: idx..idx + slice.len(),
                };
                insert_many(value, idx, slice.into_iter());
                assert!(mutator.len_range.contains(&value.len()));
                (
                    r,
                    mutator.complexity_from_inner(cache.sum_cplx + added_cplx, value.len()),
                )
            }
        }
    }
}
