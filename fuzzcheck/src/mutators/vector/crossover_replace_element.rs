use super::VecMutator;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::mutators::CrossoverStep;
use crate::{Mutator, SubValueProvider};

pub struct CrossoverReplaceElement;

#[derive(Clone)]
pub struct CrossoverReplaceElementStep<T> {
    crossover_steps: Vec<CrossoverStep<T>>,
}
pub enum ConcreteCrossoverReplaceElement<T> {
    Random(usize),
    ReplaceElement { el: T, cplx: f64, idx: usize },
}
pub enum RevertCrossoverReplaceElement<T, UT> {
    Random(UT, usize),
    ReplaceElement { el: T, idx: usize },
}

impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertCrossoverReplaceElement<T, M::UnmutateToken>
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
            RevertCrossoverReplaceElement::Random(token, idx) => {
                mutator.m.unmutate(&mut value[idx], &mut cache.inner[idx], token);
            }
            RevertCrossoverReplaceElement::ReplaceElement { mut el, idx } => std::mem::swap(&mut value[idx], &mut el),
        }
    }
}

impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for CrossoverReplaceElement
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = !;
    type Step = CrossoverReplaceElementStep<T>;
    type Concrete<'a> = ConcreteCrossoverReplaceElement<T>;
    type Revert = RevertCrossoverReplaceElement<T, M::UnmutateToken>;

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
        if mutator.m.global_search_space_complexity() == 0. {
            return None;
        }
        if value.is_empty() {
            None
        } else {
            Some(CrossoverReplaceElementStep {
                crossover_steps: vec![CrossoverStep::default(); value.len()],
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
        let value_cplx = mutator.complexity(value, cache);
        let spare_cplx = max_cplx - value_cplx;
        let choice = mutator.rng.usize(..value.len());
        let step = &mut step.crossover_steps[choice];
        if let Some((el, el_cplx)) = step.get_next_subvalue(subvalue_provider, spare_cplx) {
            if mutator.m.is_valid(el) {
                let el = el.clone();
                return Some(ConcreteCrossoverReplaceElement::ReplaceElement {
                    el,
                    cplx: el_cplx,
                    idx: choice,
                });
            }
        }
        Some(ConcreteCrossoverReplaceElement::Random(choice))
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
            ConcreteCrossoverReplaceElement::Random(idx) => {
                let old_cplx = mutator.complexity(value, cache);
                let old_el_cplx = mutator.m.complexity(&value[idx], &cache.inner[idx]);
                let spare_cplx = max_cplx - (old_cplx - old_el_cplx);
                let (token, new_el_cplx) = mutator
                    .m
                    .random_mutate(&mut value[idx], &mut cache.inner[idx], spare_cplx);
                (
                    RevertCrossoverReplaceElement::Random(token, idx),
                    mutator.complexity_from_inner(cache.sum_cplx - old_el_cplx + new_el_cplx, value.len()),
                )
            }
            ConcreteCrossoverReplaceElement::ReplaceElement { mut el, cplx, idx } => {
                let old_el_cplx = mutator.m.complexity(&value[idx], &cache.inner[idx]);
                std::mem::swap(&mut value[idx], &mut el);
                let r = RevertCrossoverReplaceElement::ReplaceElement { el, idx };
                (
                    r,
                    mutator.complexity_from_inner(cache.sum_cplx - old_el_cplx + cplx, value.len()),
                )
            }
        }
    }
}
