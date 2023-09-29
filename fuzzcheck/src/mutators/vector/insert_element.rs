use super::VecMutator;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::{Mutator, SubValueProvider};

pub struct InsertElement;

#[derive(Clone)]
pub struct InsertElementRandomStep;

#[derive(Clone)]
pub struct InsertElementStep<A> {
    arbitrary_steps: Vec<(usize, A)>,
}
pub struct ConcreteInsertElement<T> {
    el: T,
    cplx: f64,
    idx: usize,
}
pub struct RevertInsertElement {
    pub idx: usize,
}

impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertInsertElement
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

impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for InsertElement
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = InsertElementRandomStep;
    type Step = InsertElementStep<M::ArbitraryStep>;
    type Concrete<'a> = ConcreteInsertElement<T>;
    type Revert = RevertInsertElement;

    #[coverage(off)]
    fn default_random_step(&self, mutator: &VecMutator<T, M>, value: &Vec<T>) -> Option<Self::RandomStep> {
        if mutator.m.max_complexity() == 0. {
            return None;
        }
        if value.len() >= *mutator.len_range.end() {
            None
        } else {
            Some(InsertElementRandomStep)
        }
    }

    #[coverage(off)]
    fn random<'a>(
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _random_step: &Self::RandomStep,
        max_cplx: f64,
    ) -> Self::Concrete<'a> {
        let value_cplx = mutator.complexity(value, cache);
        let spare_cplx = max_cplx - value_cplx;

        let (el, cplx) = mutator.m.random_arbitrary(spare_cplx);
        ConcreteInsertElement {
            el,
            cplx,
            idx: mutator.rng.usize(..=value.len()),
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
        if value.len() >= *mutator.len_range.end() {
            None
        } else {
            Some(InsertElementStep {
                arbitrary_steps: (0..=value.len())
                    .map(
                        #[coverage(off)]
                        |i| (i, mutator.m.default_arbitrary_step()),
                    )
                    .collect(),
            })
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
        if step.arbitrary_steps.is_empty() {
            return None;
        }
        let value_cplx = mutator.complexity(value, cache);
        let spare_cplx = max_cplx - value_cplx;
        let choice = mutator.rng.usize(..step.arbitrary_steps.len());
        let (idx, arbitrary_step) = &mut step.arbitrary_steps[choice];

        if let Some((el, cplx)) = mutator.m.ordered_arbitrary(arbitrary_step, spare_cplx) {
            Some(ConcreteInsertElement { el, cplx, idx: *idx })
        } else {
            step.arbitrary_steps.remove(choice);
            Self::from_step(mutator, value, cache, step, subvalue_provider, max_cplx)
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
        (RevertInsertElement { idx: mutation.idx }, new_cplx)
    }
}
