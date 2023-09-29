use super::VecMutator;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::{Mutator, SubValueProvider};

pub struct MutateElement;

#[derive(Clone)]
pub struct MutateElementRandomStep;

#[derive(Clone)]
pub struct MutateElementStep<S> {
    pub indices: Vec<usize>,
    pub inner_steps: Vec<S>,
}
pub enum ConcreteMutateElement<'a, S> {
    Random {
        el_idx: usize,
    },
    Ordered {
        step_idx: usize,
        step: &'a mut MutateElementStep<S>,
    },
}

pub struct RevertMutateElement<U> {
    pub idx: usize,
    pub unmutate_token: Option<U>,
}

impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertMutateElement<M::UnmutateToken>
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
        let Self { idx, unmutate_token } = self;
        if let Some(unmutate_token) = unmutate_token {
            mutator
                .m
                .unmutate(&mut value[idx], &mut cache.inner[self.idx], unmutate_token)
        }
    }
}

impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for MutateElement
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = MutateElementRandomStep;
    type Step = MutateElementStep<M::MutationStep>;
    type Concrete<'a> = ConcreteMutateElement<'a, M::MutationStep>;
    type Revert = RevertMutateElement<M::UnmutateToken>;

    #[coverage(off)]
    fn default_random_step(&self, mutator: &VecMutator<T, M>, value: &Vec<T>) -> Option<Self::RandomStep> {
        if mutator.m.max_complexity() == 0. {
            return None;
        }
        if value.is_empty() {
            None
        } else {
            Some(MutateElementRandomStep)
        }
    }

    #[coverage(off)]
    fn random<'a>(
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _random_step: &Self::RandomStep,
        _max_cplx: f64,
    ) -> Self::Concrete<'a> {
        ConcreteMutateElement::Random {
            el_idx: mutator.rng.usize(..value.len()),
        }
    }

    #[coverage(off)]
    fn default_step(
        &self,
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) -> Option<Self::Step> {
        if mutator.m.max_complexity() == 0. {
            return None;
        }
        if value.is_empty() {
            None
        } else {
            let inner_steps = value
                .iter()
                .zip(cache.inner.iter())
                .map(
                    #[coverage(off)]
                    |(v, c)| mutator.m.default_mutation_step(v, c),
                )
                .collect();
            Some(MutateElementStep {
                indices: (0..value.len()).collect(),
                inner_steps,
            })
        }
    }

    #[coverage(off)]
    fn from_step<'a>(
        mutator: &VecMutator<T, M>,
        _value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        step: &'a mut Self::Step,
        _subvalue_provider: &dyn SubValueProvider,
        _max_cplx: f64,
    ) -> Option<Self::Concrete<'a>> {
        if step.indices.is_empty() {
            None
        } else {
            // no! should be chosen from a vose alias!
            let step_idx = mutator.rng.usize(..step.indices.len());

            Some(ConcreteMutateElement::Ordered { step_idx, step })
        }
    }
    #[coverage(off)]
    fn apply<'a>(
        mutation: Self::Concrete<'a>,
        mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        subvalue_provider: &dyn SubValueProvider,
        max_cplx: f64,
    ) -> (Self::Revert, f64) {
        let value_cplx = mutator.complexity(value, cache);

        let el_idx = match &mutation {
            ConcreteMutateElement::Random { el_idx } => *el_idx,
            ConcreteMutateElement::Ordered { step_idx, step } => step.indices[*step_idx],
        };
        let el = &mut value[el_idx];
        let el_cache = &mut cache.inner[el_idx];
        let el_cplx = mutator.m.complexity(el, el_cache);
        let spare_cplx = max_cplx - value_cplx;
        let max_el_cplx = spare_cplx + el_cplx;
        // the vose alias should be accessible here, through the concrete mutation,
        // so that we can remove the elements whose mutations are exhausted

        match mutation {
            ConcreteMutateElement::Random { el_idx: _ } => {
                let (t, new_el_cplx) = mutator.m.random_mutate(el, el_cache, max_el_cplx);
                let new_cplx = mutator.complexity_from_inner(cache.sum_cplx - el_cplx + new_el_cplx, value.len());
                (
                    RevertMutateElement {
                        idx: el_idx,
                        unmutate_token: Some(t),
                    },
                    new_cplx,
                )
            }
            ConcreteMutateElement::Ordered { step_idx, step } => {
                // it can't not succeed!!!!!!!
                // maybe ignore it,
                // accept that a duplicate test case will be sent
                // it shouldn't happen very often
                // ideally the Mutator trait should have the same RandomStep / OrderedStep / ConcreteMutation / Revert concepts
                // it has OrderedStep and Revert but is missing RandomStep (bad name) and ConcreteMutation
                let el_step = &mut step.inner_steps[el_idx];
                if let Some((t, new_el_cplx)) =
                    mutator
                        .m
                        .ordered_mutate(el, el_cache, el_step, subvalue_provider, max_el_cplx)
                {
                    let new_cplx = mutator.complexity_from_inner(cache.sum_cplx - el_cplx + new_el_cplx, value.len());

                    (
                        RevertMutateElement {
                            idx: el_idx,
                            unmutate_token: Some(t),
                        },
                        new_cplx,
                    )
                } else {
                    step.indices.remove(step_idx);
                    (
                        RevertMutateElement {
                            idx: el_idx,
                            unmutate_token: None,
                        },
                        value_cplx,
                    )
                }
            }
        }
    }
}
