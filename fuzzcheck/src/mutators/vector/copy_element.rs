use super::VecMutator;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::{Mutator, SubValueProvider};

pub struct CopyElement;

#[derive(Clone)]
pub struct CopyElementRandomStep;

#[derive(Clone)]
pub struct CopyElementStep {
    from_idx: usize,
    to_idx: usize,
}
pub struct ConcreteCopyElement<T> {
    el: T,
    cplx: f64,
    idx: usize,
}
pub struct RevertCopyElement {
    idx: usize,
}

impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertCopyElement
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[coverage(off)]
    fn revert(
        self,
        _mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) {
        let _ = value.remove(self.idx);
    }
}

impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for CopyElement
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = CopyElementRandomStep;
    type Step = CopyElementStep;
    type Concrete<'a> = ConcreteCopyElement<T>;
    type Revert = RevertCopyElement;

    #[coverage(off)]
    fn default_random_step(&self, mutator: &VecMutator<T, M>, value: &Vec<T>) -> Option<Self::RandomStep> {
        if mutator.m.max_complexity() == 0. {
            return None;
        }
        if value.is_empty() || value.len() >= *mutator.len_range.end() {
            None
        } else {
            Some(CopyElementRandomStep)
        }
    }

    #[coverage(off)]
    fn random<'a>(
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _random_step: &Self::RandomStep,
        _max_cplx: f64,
    ) -> Self::Concrete<'a> {
        let from_idx = mutator.rng.usize(..value.len());
        let to_idx = mutator.rng.usize(..value.len());

        let (el, el_cache) = (&value[from_idx], &cache.inner[from_idx]);
        let cplx = mutator.m.complexity(el, el_cache);

        ConcreteCopyElement {
            el: el.clone(),
            cplx,
            idx: to_idx,
        }
    }

    #[coverage(off)]
    fn default_step(
        &self,
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) -> Option<Self::Step> {
        if mutator.m.max_complexity() == 0. {
            return None;
        }
        if value.is_empty() || value.len() >= *mutator.len_range.end() {
            None
        } else {
            Some(Self::Step { from_idx: 0, to_idx: 0 })
        }
    }

    #[coverage(off)]
    fn from_step<'a>(
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        step: &'a mut Self::Step,
        subvalue_provider: &dyn SubValueProvider,
        max_cplx: f64,
    ) -> Option<Self::Concrete<'a>> {
        // The step.from_idx increments from 0 to value.len()
        // once it reaches value.len(), we have tried every single possibility
        if step.from_idx == value.len() {
            return None;
        }
        // It doesn't make sense to copy an element to its same index
        // e.g. [0, 1, 2, 3]
        // copy 0 to 0 -> [0, 0, 1, 2, 3]
        // ok, but then:
        // copy 0 to 1 -> [0, 0, 1, 2, 3]
        // they're the same thing
        if step.from_idx == step.to_idx {
            step.to_idx += 1;
        }

        let value_cplx = mutator.complexity(value, cache);
        let spare_cplx = max_cplx - value_cplx;

        let (el, el_cache) = (&value[step.from_idx], &cache.inner[step.from_idx]);
        let cplx = mutator.m.complexity(el, el_cache);

        // cannot copy an element that would make the value exceed the maximum complexity
        // so we try another one
        if cplx > spare_cplx {
            step.from_idx += 1;
            step.to_idx = 0;
            Self::from_step(mutator, value, cache, step, subvalue_provider, max_cplx)
        } else {
            let concrete = ConcreteCopyElement {
                el: el.clone(),
                cplx,
                idx: step.to_idx,
            };
            step.to_idx = (step.to_idx + 1) % (value.len() + 1);
            if step.to_idx == 0 {
                // then we have tried copying the element at from_idx to every other index
                // time to copy a different element
                step.from_idx += 1;
            }
            Some(concrete)
        }
    }

    #[coverage(off)]
    fn apply<'a>(
        mutation: Self::Concrete<'a>,
        mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _subvalue_provider: &dyn SubValueProvider,
        _max_cplx: f64,
    ) -> (Self::Revert, f64) {
        value.insert(mutation.idx, mutation.el);
        let new_cplx = mutator.complexity_from_inner(cache.sum_cplx + mutation.cplx, value.len());
        (RevertCopyElement { idx: mutation.idx }, new_cplx)
    }
}
