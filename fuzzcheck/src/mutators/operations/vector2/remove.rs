use crate::{
    mutators::operations::{Mutation, RevertMutation},
    Mutator,
};

use super::VecM;

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

impl<T, M> RevertMutation<Vec<T>, VecM<T, M>> for RevertRemove<T>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    fn revert(self, _mutator: &VecM<T, M>, value: &mut Vec<T>, _cache: &mut <VecM<T, M> as Mutator<Vec<T>>>::Cache) {
        value.insert(self.idx, self.element);
    }
}

impl<T, M> Mutation<Vec<T>, VecM<T, M>> for Remove
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = ();
    type Step = RemoveStep;
    type Concrete<'a> = ConcreteRemove;
    type Revert = RevertRemove<T>;

    fn default_random_step(_mutator: &VecM<T, M>, value: &Vec<T>) -> Option<Self::RandomStep> {
        if value.is_empty() {
            None
        } else {
            Some(())
        }
    }

    fn random<'a>(
        mutator: &VecM<T, M>,
        value: &Vec<T>,
        _cache: &<VecM<T, M> as Mutator<Vec<T>>>::Cache,
        _random_step: &Self::RandomStep,
        _max_cplx: f64,
    ) -> Self::Concrete<'a> {
        ConcreteRemove {
            idx: mutator.rng.usize(..value.len()),
        }
    }

    fn default_step(
        mutator: &VecM<T, M>,
        value: &Vec<T>,
        _cache: &<VecM<T, M> as Mutator<Vec<T>>>::Cache,
    ) -> Option<Self::Step> {
        if *mutator.len_range.start() < value.len() {
            Some(RemoveStep { idx: 0 })
        } else {
            None
        }
    }

    fn from_step<'a>(
        _mutator: &VecM<T, M>,
        value: &Vec<T>,
        _cache: &<VecM<T, M> as Mutator<Vec<T>>>::Cache,
        step: &'a mut Self::Step,
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

    fn apply<'a>(
        mutation: Self::Concrete<'a>,
        mutator: &VecM<T, M>,
        value: &mut Vec<T>,
        cache: &mut <VecM<T, M> as Mutator<Vec<T>>>::Cache,
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
