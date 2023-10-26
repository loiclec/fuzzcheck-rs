use super::VecMutator;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::{Mutator, SubValueProvider};

pub struct SwapElements;

// for now, everything random
// but could decide to which two elements to swap
#[derive(Clone)]
pub struct SwapElementsStep {
    idx_1: usize,
    idx_2: usize,
}
pub struct ConcreteSwapElements {
    idx_1: usize,
    idx_2: usize,
}
pub struct RevertSwapElements {
    idx_1: usize,
    idx_2: usize,
}

impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertSwapElements
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
        value.swap(self.idx_1, self.idx_2);
    }
}

impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for SwapElements
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = SwapElementsStep;
    type Step = SwapElementsStep;
    type Concrete<'a> = ConcreteSwapElements;
    type Revert = RevertSwapElements;
    #[coverage(off)]
    fn default_random_step(&self, mutator: &VecMutator<T, M>, value: &Vec<T>) -> Option<Self::RandomStep> {
        if mutator.m.max_complexity() == 0. {
            return None;
        }
        if value.len() <= 1 {
            None
        } else {
            let idx_1 = mutator.rng.usize(..value.len());
            let choice_other = mutator.rng.usize(..value.len() - 1);
            let idx_2 = if choice_other < idx_1 {
                choice_other
            } else {
                choice_other + 1
            };
            Some(SwapElementsStep { idx_1, idx_2 })
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
        ConcreteSwapElements {
            idx_1: random_step.idx_1,
            idx_2: random_step.idx_2,
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
        if value.len() <= 1 {
            None
        } else {
            Some(SwapElementsStep { idx_1: 0, idx_2: 1 })
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
        if step.idx_1 >= value.len() - 1 {
            None
        } else {
            let x = ConcreteSwapElements {
                idx_1: step.idx_1,
                idx_2: step.idx_2,
            };
            step.idx_2 += 1;
            if step.idx_2 == value.len() {
                step.idx_1 += 1;
                step.idx_2 = step.idx_1 + 1;
            }
            Some(x)
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
        let cplx = mutator.complexity(value, cache);
        value.swap(mutation.idx_1, mutation.idx_2);
        (
            RevertSwapElements {
                idx_1: mutation.idx_1,
                idx_2: mutation.idx_2,
            },
            cplx,
        )
    }
}
