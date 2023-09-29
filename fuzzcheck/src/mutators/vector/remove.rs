use super::VecMutator;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::{Mutator, SubValueProvider};

pub struct Remove;

#[derive(Clone)]
pub struct RemoveStep {
    pub idx: usize,
}

pub struct ConcreteRemove {
    pub idx: usize,
}
pub struct RevertRemove<T> {
    pub idx: usize,
    pub element: T,
}

impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertRemove<T>
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
        value.insert(self.idx, self.element);
    }
}

impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for Remove
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = RemoveStep;
    type Step = RemoveStep;
    type Concrete<'a> = ConcreteRemove;
    type Revert = RevertRemove<T>;

    #[coverage(off)]
    fn default_random_step(&self, mutator: &VecMutator<T, M>, value: &Vec<T>) -> Option<Self::RandomStep> {
        if mutator.m.max_complexity() == 0. {
            return None;
        }
        if value.len() <= *mutator.len_range.start() {
            None
        } else {
            Some(RemoveStep {
                idx: mutator.rng.usize(..value.len()),
            })
        }
    }
    #[coverage(off)]
    fn random<'a>(
        _mutator: &VecMutator<T, M>,
        _value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        random_step: &Self::RandomStep,
        _max_cplx: f64,
    ) -> Self::Concrete<'a> {
        ConcreteRemove { idx: random_step.idx }
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
        if value.len() <= *mutator.len_range.start() {
            None
        } else {
            Some(RemoveStep { idx: 0 })
        }
    }
    #[coverage(off)]
    fn from_step<'a>(
        _mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        step: &'a mut Self::Step,
        _subvalue_provider: &dyn SubValueProvider,
        _max_cplx: f64,
    ) -> Option<Self::Concrete<'a>> {
        if step.idx < value.len() {
            let x = ConcreteRemove { idx: step.idx };
            step.idx += 1;
            Some(x)
        } else {
            None
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
        let removed = value.remove(mutation.idx);
        let removed_cplx = mutator.m.complexity(&removed, &cache.inner[mutation.idx]);
        let new_cplx = mutator.complexity_from_inner(cache.sum_cplx - removed_cplx, value.len());
        (
            RevertRemove {
                idx: mutation.idx,
                element: removed,
            },
            new_cplx,
        )
    }
}
