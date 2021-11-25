use crate::{
    fenwick_tree::FenwickTree,
    mutators::{
        operations::{vector2::remove, Mutation, RevertMutation},
        vose_alias::VoseAlias,
    },
    Mutator,
};

use super::{mutate_element, VecM};

pub struct VectorMutation;

#[derive(Clone)]
pub enum VectorMutationInnerStep<S> {
    Remove(remove::RemoveStep),
    MutateElement(mutate_element::MutateElementStep<S>),
}
#[derive(Clone)]
pub enum VectorMutationInnerRandomStep {
    Remove,
    MutateElement,
}
#[derive(Clone)]
pub struct VectorMutationStep<S> {
    inner_steps: Vec<VectorMutationInnerStep<S>>,
    weights_and_times_chosen: Vec<(f64, f64)>,
    sampling: FenwickTree,
}
#[derive(Clone)]
pub struct VectorMutationRandomStep {
    inner_steps: Vec<VectorMutationInnerRandomStep>,
    sampling: VoseAlias,
}
pub enum ConcreteVectorMutation<'a, S> {
    Remove(remove::ConcreteRemove),
    MutateElement(mutate_element::ConcreteMutateElement<'a, S>),
}
pub enum RevertVectorMutation<T, U> {
    Remove(remove::RevertRemove<T>),
    MutateElement(mutate_element::RevertMutateElement<U>),
}
impl<T, M> RevertMutation<Vec<T>, VecM<T, M>> for RevertVectorMutation<T, M::UnmutateToken>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    fn revert(self, mutator: &VecM<T, M>, value: &mut Vec<T>, cache: &mut <VecM<T, M> as Mutator<Vec<T>>>::Cache) {
        match self {
            RevertVectorMutation::Remove(r) => r.revert(mutator, value, cache),
            RevertVectorMutation::MutateElement(r) => r.revert(mutator, value, cache),
        }
    }
}

impl<T, M> Mutation<Vec<T>, VecM<T, M>> for VectorMutation
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = VectorMutationRandomStep;
    type Step = VectorMutationStep<M::MutationStep>;
    type Concrete<'a> = ConcreteVectorMutation<'a, M::MutationStep>;
    type Revert = RevertVectorMutation<T, M::UnmutateToken>;

    fn default_random_step(mutator: &VecM<T, M>, value: &Vec<T>) -> Option<Self::RandomStep> {
        let remove_step =
            remove::Remove::default_random_step(mutator, value).map(|_| VectorMutationInnerRandomStep::Remove);

        let inner_steps_and_weights = [
            (remove_step, 10.), // remove
        ]
        .into_iter()
        .filter_map(|(x, y)| x.map(|x| (x, y)))
        .collect::<Vec<_>>();

        if inner_steps_and_weights.is_empty() {
            return None;
        }

        let sum_weights: f64 = inner_steps_and_weights.iter().map(|x| x.1).sum();
        let probabilities = inner_steps_and_weights.iter().map(|x| x.1 / sum_weights).collect();
        let sampling = VoseAlias::new(probabilities);
        let inner_steps = inner_steps_and_weights.into_iter().map(|x| x.0).collect();

        Some(VectorMutationRandomStep { inner_steps, sampling })
    }

    fn random<'a>(
        mutator: &VecM<T, M>,
        value: &Vec<T>,
        cache: &<VecM<T, M> as Mutator<Vec<T>>>::Cache,
        step: &Self::RandomStep,
        max_cplx: f64,
    ) -> Self::Concrete<'a> {
        let inner_step_idx = step.sampling.sample();
        let step = &step.inner_steps[inner_step_idx];
        match step {
            VectorMutationInnerRandomStep::Remove => {
                ConcreteVectorMutation::Remove(remove::Remove::random(mutator, value, cache, &(), max_cplx))
            }
            VectorMutationInnerRandomStep::MutateElement => ConcreteVectorMutation::MutateElement(
                mutate_element::MutateElement::random(mutator, value, cache, &(), max_cplx),
            ),
        }
    }

    fn default_step(
        mutator: &VecM<T, M>,
        value: &Vec<T>,
        cache: &<VecM<T, M> as Mutator<Vec<T>>>::Cache,
    ) -> Option<Self::Step> {
        let remove_step = remove::Remove::default_step(mutator, value, cache).map(VectorMutationInnerStep::Remove);
        let mutate_element_step = mutate_element::MutateElement::default_step(mutator, value, cache)
            .map(VectorMutationInnerStep::MutateElement);

        let inner_steps_and_weights = [(remove_step, 1000., 1.), (mutate_element_step, 10., 1.)]
            .into_iter()
            .filter_map(|(x, y, z)| x.map(|x| (x, y, z)))
            .collect::<Vec<_>>();

        if inner_steps_and_weights.is_empty() {
            return None;
        }

        let mut inner_steps = Vec::with_capacity(inner_steps_and_weights.len());
        let mut weights_and_times_chosen = Vec::with_capacity(inner_steps_and_weights.len());
        let mut probabilities = Vec::with_capacity(inner_steps_and_weights.len());
        for (inner_step, weight, times_chosen) in inner_steps_and_weights {
            inner_steps.push(inner_step);
            probabilities.push(weight / times_chosen);
            weights_and_times_chosen.push((weight, times_chosen));
        }
        let sampling = FenwickTree::new(probabilities);

        Some(VectorMutationStep {
            inner_steps,
            weights_and_times_chosen,
            sampling,
        })
    }

    fn from_step<'a>(
        mutator: &VecM<T, M>,
        value: &Vec<T>,
        cache: &<VecM<T, M> as Mutator<Vec<T>>>::Cache,
        step: &'a mut Self::Step,
        max_cplx: f64,
    ) -> Option<Self::Concrete<'a>> {
        let inner_step_idx = step.sampling.sample(&mutator.rng)?;

        let inner_step = &mut step.inner_steps[inner_step_idx];

        let concrete = match inner_step {
            VectorMutationInnerStep::Remove(step) => {
                remove::Remove::from_step(mutator, value, cache, step, max_cplx).map(ConcreteVectorMutation::Remove)
            }
            VectorMutationInnerStep::MutateElement(step) => {
                mutate_element::MutateElement::from_step(mutator, value, cache, step, max_cplx)
                    .map(ConcreteVectorMutation::MutateElement)
            }
        };

        if let Some(concrete) = concrete {
            // udpate the probabilities
            let (weight, times_chosen) = &mut step.weights_and_times_chosen[inner_step_idx];
            let old_probability = *weight / *times_chosen;
            *times_chosen += 1.0;
            let new_probability = *weight / *times_chosen;
            step.sampling.update(inner_step_idx, new_probability - old_probability);
            let concrete = unsafe { std::mem::transmute::<_, ConcreteVectorMutation<'a, M::MutationStep>>(concrete) };
            return Some(concrete);
        }
        // remove the step from the array
        step.weights_and_times_chosen.remove(inner_step_idx);
        step.inner_steps.remove(inner_step_idx);
        let probabilities = step.weights_and_times_chosen.iter().map(|(w, df)| w / df).collect();
        step.sampling = FenwickTree::new(probabilities);
        // Self::from_step(mutator, value, cache, step, max_cplx)
        todo!()
    }

    fn apply<'a>(
        mutation: Self::Concrete<'a>,
        mutator: &VecM<T, M>,
        value: &mut Vec<T>,
        cache: &mut <VecM<T, M> as Mutator<Vec<T>>>::Cache,
        max_cplx: f64,
    ) -> (Self::Revert, f64) {
        match mutation {
            ConcreteVectorMutation::Remove(mutation) => {
                let (revert, cplx) = remove::Remove::apply(mutation, mutator, value, cache, max_cplx);
                (RevertVectorMutation::Remove(revert), cplx)
            }
            ConcreteVectorMutation::MutateElement(mutation) => {
                let (revert, cplx) = mutate_element::MutateElement::apply(mutation, mutator, value, cache, max_cplx);
                (RevertVectorMutation::MutateElement(revert), cplx)
            }
        }
    }
}
