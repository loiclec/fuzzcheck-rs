use super::crossover_insert_slice::CrossoverInsertSlice;
use super::crossover_replace_element::CrossoverReplaceElement;
use super::{
    arbitrary, copy_element, crossover_insert_slice, crossover_replace_element, insert_element, insert_many_elements,
    mutate_element, only_choose_length, remove, remove_and_insert_element, swap_elements, VecMutator,
};
use crate::mutators::mutations::{Mutation, NoMutation, RevertMutation};
use crate::mutators::vose_alias::VoseAlias;
use crate::Mutator;

pub struct WeightedMutation<M> {
    mutation: M,
    random_weight: f64,
    ordered_weight: f64,
}
macro_rules! impl_vec_mutation {
    ($(($i:ident,$t:ty)),*) => {
        pub enum InnerVectorMutation {
            $($i($t),)*
        }
        pub struct VectorMutation {
            mutations: Vec<WeightedMutation<InnerVectorMutation>>,
        }
        pub enum VectorMutationInnerStep<T, M>
        where
            T: Clone + 'static,
            M: Mutator<T>,
        {
            $($i(< $t as Mutation<Vec<T>, VecMutator<T, M>>>::Step),)*
        }
        pub struct VectorMutationStep<T, M>
        where
            T: Clone + 'static,
            M: Mutator<T>,
        {
            inner_steps: Vec<VectorMutationInnerStep<T, M>>,
            weights: Vec<f64>,
            sampling: VoseAlias,
        }
        pub enum VectorMutationInnerRandomStep<T, M>
        where
            T: Clone + 'static,
            M: Mutator<T>,
        {
            $($i(< $t as Mutation<Vec<T>, VecMutator<T, M>>>::RandomStep),)*
        }
        pub struct VectorMutationRandomStep<T, M>
        where
            T: Clone + 'static,
            M: Mutator<T>
        {
            inner_steps: Vec<VectorMutationInnerRandomStep<T, M>>,
            sampling: VoseAlias,
        }
        pub enum ConcreteVectorMutation<'a, T, M>
        where
            T: Clone + 'static,
            M: Mutator<T>,
        {
            $($i(< $t as Mutation<Vec<T>, VecMutator<T, M>>>::Concrete<'a>),)*
        }
        pub enum RevertVectorMutation<T, M>
        where
            T: Clone + 'static,
            M: Mutator<T>,
        {
            $($i(< $t as Mutation<Vec<T>, VecMutator<T, M>>>::Revert),)*
        }
        impl<T, M> Clone for VectorMutationInnerStep<T, M>
        where
            T: Clone + 'static,
            M: Mutator<T>,
        {
            #[coverage(off)]
            fn clone(&self) -> Self {
                match self {
                    $(
                        Self::$i(x) => Self::$i(x.clone())
                    ),*
                }
            }
        }

        impl<T, M> Clone for VectorMutationStep<T, M>
        where
            T: Clone + 'static,
            M: Mutator<T>,
            VectorMutationInnerStep<T, M>: Clone
        {
            #[coverage(off)]
            fn clone(&self) -> Self {
                Self {
                    inner_steps: self.inner_steps.clone(),
                    weights: self.weights.clone(),
                    sampling: self.sampling.clone(),
                }
            }
        }

        impl<T, M> Clone for VectorMutationInnerRandomStep<T, M>
        where
            T: Clone + 'static,
            M: Mutator<T>,
        {
            #[allow(unreachable_code)]
            #[coverage(off)]
            fn clone(&self) -> Self {
                match self {
                    $(
                        Self::$i(x) => Self::$i(x.clone())
                    ),*
                }
            }
        }

        impl<T, M> Clone for VectorMutationRandomStep<T, M>
        where
            T: Clone + 'static,
            M: Mutator<T>,
            VectorMutationInnerRandomStep<T, M>: Clone
        {
            #[coverage(off)]
            fn clone(&self) -> Self {
                Self {
                    inner_steps: self.inner_steps.clone(),
                    sampling: self.sampling.clone(),
                }
            }
        }

        impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertVectorMutation<T, M>
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
                    $(
                        Self::$i(r) => r.revert(mutator, value, cache)
                    ),*
                }
            }
        }

        impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for VectorMutation
        where
            T: Clone + 'static,
            M: Mutator<T>,
        {
            type RandomStep = VectorMutationRandomStep<T, M>;
            type Step = VectorMutationStep<T, M>;
            type Concrete<'a> = ConcreteVectorMutation<'a, T, M>;
            type Revert = RevertVectorMutation<T, M>;
            #[coverage(off)]
            fn default_random_step(&self, mutator: &VecMutator<T, M>, value: &Vec<T>) -> Option<Self::RandomStep> {
                let inner_steps_and_weights: Vec<(_, f64)> = self
                    .mutations
                    .iter()
                    .filter_map(#[coverage(off)] |mutation| {
                        match &mutation.mutation {
                            $(
                                InnerVectorMutation::$i(r) => r
                                    .default_random_step(mutator, value)
                                    .map(VectorMutationInnerRandomStep::$i)
                            ),*
                        }
                        .map(#[coverage(off)] |inner| (inner, mutation.random_weight))
                    })
                    .collect::<Vec<_>>();

                if inner_steps_and_weights.is_empty() {
                    return None;
                }
                let weights = inner_steps_and_weights.iter().map(#[coverage(off)] |x| x.1).collect();
                let sampling = VoseAlias::new(weights);
                let inner_steps = inner_steps_and_weights.into_iter().map(#[coverage(off)] |x| x.0).collect();

                Some(VectorMutationRandomStep { inner_steps, sampling })
            }
            #[coverage(off)]
            fn random<'a>(
                mutator: &VecMutator<T, M>,
                value: &Vec<T>,
                cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
                step: &Self::RandomStep,
                max_cplx: f64,
            ) -> Self::Concrete<'a> {
                let inner_step_idx = step.sampling.sample();
                let step = &step.inner_steps[inner_step_idx];
                match step {
                    $(
                        VectorMutationInnerRandomStep::$i(s) =>
                            ConcreteVectorMutation::$i(<$t>::random(mutator, value, cache, s, max_cplx))
                    ),*
                }
            }
            #[coverage(off)]
            fn default_step(
                &self,
                mutator: &VecMutator<T, M>,
                value: &Vec<T>,
                cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
            ) -> Option<Self::Step> {
                let inner_steps_and_weights: Vec<(VectorMutationInnerStep<_, _>, f64)> = self
                    .mutations
                    .iter()
                    .filter_map(#[coverage(off)] |mutation| {
                        match &mutation.mutation {
                            $(
                                InnerVectorMutation::$i(r) =>
                                    r.default_step(mutator, value, cache)
                                    .map(VectorMutationInnerStep::$i)
                            ),*
                        }
                        .map(#[coverage(off)] |inner| (inner, mutation.ordered_weight))
                    })
                    .collect::<Vec<_>>();

                if inner_steps_and_weights.is_empty() {
                    return None;
                }

                let mut inner_steps = Vec::with_capacity(inner_steps_and_weights.len());
                let mut weights = Vec::with_capacity(inner_steps_and_weights.len());
                let mut probabilities = Vec::with_capacity(inner_steps_and_weights.len());
                for (inner_step, weight) in inner_steps_and_weights {
                    inner_steps.push(inner_step);
                    probabilities.push(weight);
                    weights.push(weight);
                }
                let sampling = VoseAlias::new(probabilities);

                Some(VectorMutationStep {
                    inner_steps,
                    weights,
                    sampling,
                })
            }
            #[coverage(off)]
            fn from_step<'a>(
                mutator: &VecMutator<T, M>,
                value: &Vec<T>,
                cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
                step: &'a mut Self::Step,
                subvalue_provider: &dyn crate::SubValueProvider,
                max_cplx: f64,
            ) -> Option<Self::Concrete<'a>> {
                if step.inner_steps.is_empty() {
                    return None;
                }
                let inner_step_idx = step.sampling.sample();
                let step_raw = step as *mut Self::Step;
                {
                    let inner_step = &mut step.inner_steps[inner_step_idx];

                    let concrete: Option<<VectorMutation as Mutation<Vec<T>, VecMutator<T, M>>>::Concrete<'a>> =
                        match inner_step {
                            $(
                                VectorMutationInnerStep::$i(step) =>
                                    <$t>::from_step(mutator, value, cache, step, subvalue_provider, max_cplx)
                                    .map(ConcreteVectorMutation::$i)
                            ),*
                        };
                    if let Some(concrete) = concrete {
                        return Some(concrete);
                    }
                }
                // See: https://stackoverflow.com/questions/50519147/double-mutable-borrow-error-in-a-loop-happens-even-with-nll-on/50570026#50570026 and https://twitter.com/m_ou_se/status/1463606672807632900 https://github.com/rust-lang/rfcs/blob/master/text/2094-nll.md#problem-case-3-conditional-control-flow-across-functions for why I had to lie to the borrow checker here
                let step = unsafe { &mut *step_raw };
                // remove the step from the array
                step.weights.remove(inner_step_idx);
                step.inner_steps.remove(inner_step_idx);
                if step.weights.is_empty() {
                    None
                } else {
                    step.sampling = VoseAlias::new(step.weights.clone());
                    Self::from_step(mutator, value, cache, step, subvalue_provider, max_cplx)
                }
            }
            #[coverage(off)]
            fn apply<'a>(
                mutation: Self::Concrete<'a>,
                mutator: &VecMutator<T, M>,
                value: &mut Vec<T>,
                cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
                subvalue_provider: &dyn crate::SubValueProvider,
                max_cplx: f64,
            ) -> (Self::Revert, f64) {
                match mutation {
                    $(
                        ConcreteVectorMutation::$i(mutation) => {
                            let (revert, cplx) =  <$t>::apply(mutation, mutator, value, cache, subvalue_provider, max_cplx);
                            (RevertVectorMutation::$i(revert), cplx)
                        }
                    )*
                }
            }
        }
    }
}

impl_vec_mutation! {
    (NoMutation, NoMutation),
    (CopyElement, copy_element::CopyElement),
    (Remove, remove::Remove),
    (MutateElement, mutate_element::MutateElement),
    (InsertElement, insert_element::InsertElement),
    (SwapElements, swap_elements::SwapElements),
    (InsertManyElements, insert_many_elements::InsertManyElements),
    (RemoveAndInsertElement, remove_and_insert_element::RemoveAndInsertElement),
    (OnlyChooseLength, only_choose_length::OnlyChooseLength),
    (Arbitrary, arbitrary::Arbitrary),
    (CrossoverReplaceElement, crossover_replace_element::CrossoverReplaceElement),
    (CrossoverInsertSlice, crossover_insert_slice::CrossoverInsertSlice)
}

impl<'a, T, M> std::fmt::Debug for ConcreteVectorMutation<'a, T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConcreteVectorMutation::NoMutation(_) => {
                write!(f, "NoMutation")
            }
            ConcreteVectorMutation::CopyElement(_) => {
                write!(f, "CopyElement")
            }
            ConcreteVectorMutation::Remove(_) => {
                write!(f, "Remove")
            }
            ConcreteVectorMutation::MutateElement(_) => {
                write!(f, "MutateElement")
            }
            ConcreteVectorMutation::InsertElement(_) => {
                write!(f, "InsertElement")
            }
            ConcreteVectorMutation::SwapElements(_) => {
                write!(f, "SwapElements")
            }
            ConcreteVectorMutation::InsertManyElements(_) => {
                write!(f, "InsertManyElements")
            }
            ConcreteVectorMutation::RemoveAndInsertElement(_) => {
                write!(f, "RemoveAndInsertElement")
            }
            ConcreteVectorMutation::OnlyChooseLength(_) => {
                write!(f, "OnlyChooseLength")
            }
            ConcreteVectorMutation::Arbitrary(_) => {
                write!(f, "Arbitrary")
            }
            ConcreteVectorMutation::CrossoverReplaceElement(_) => {
                write!(f, "CrossoverReplaceElement")
            }
            ConcreteVectorMutation::CrossoverInsertSlice(_) => {
                write!(f, "CrossoverInsertSlice")
            }
        }
    }
}

// ====== Default Vector Mutations =====

impl Default for VectorMutation {
    #[coverage(off)]
    fn default() -> Self {
        // use the same standard for all of them
        Self {
            mutations: vec![
                WeightedMutation {
                    mutation: InnerVectorMutation::CopyElement(copy_element::CopyElement),
                    random_weight: 50.,
                    ordered_weight: 500.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::OnlyChooseLength(only_choose_length::OnlyChooseLength),
                    random_weight: 1., // doesn't matter, it's the only mutation when relevant!
                    ordered_weight: 1.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::Arbitrary(arbitrary::Arbitrary),
                    random_weight: 1.,
                    ordered_weight: 1.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::Remove(remove::Remove),
                    random_weight: 50.,
                    ordered_weight: 50_000.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::MutateElement(mutate_element::MutateElement),
                    random_weight: 1000.,
                    ordered_weight: 1000.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::InsertElement(insert_element::InsertElement),
                    random_weight: 50.,
                    ordered_weight: 50.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::RemoveAndInsertElement(
                        remove_and_insert_element::RemoveAndInsertElement,
                    ),
                    random_weight: 50.,
                    ordered_weight: 30.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::SwapElements(swap_elements::SwapElements),
                    random_weight: 20.,
                    ordered_weight: 500.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::InsertManyElements(insert_many_elements::InsertManyElements {
                        nbr_added_elements: 2,
                        repeated: false,
                    }),
                    random_weight: 10.,
                    ordered_weight: 5.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::InsertManyElements(insert_many_elements::InsertManyElements {
                        nbr_added_elements: 3,
                        repeated: false,
                    }),
                    random_weight: 8.,
                    ordered_weight: 4.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::InsertManyElements(insert_many_elements::InsertManyElements {
                        nbr_added_elements: 4,
                        repeated: false,
                    }),
                    random_weight: 6.,
                    ordered_weight: 3.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::InsertManyElements(insert_many_elements::InsertManyElements {
                        nbr_added_elements: 5,
                        repeated: false,
                    }),
                    random_weight: 4.,
                    ordered_weight: 2.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::InsertManyElements(insert_many_elements::InsertManyElements {
                        nbr_added_elements: 2,
                        repeated: true,
                    }),
                    random_weight: 10.,
                    ordered_weight: 5.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::InsertManyElements(insert_many_elements::InsertManyElements {
                        nbr_added_elements: 3,
                        repeated: true,
                    }),
                    random_weight: 8.,
                    ordered_weight: 4.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::CrossoverReplaceElement(CrossoverReplaceElement),
                    random_weight: 0.,
                    ordered_weight: 100.,
                },
                WeightedMutation {
                    mutation: InnerVectorMutation::CrossoverInsertSlice(CrossoverInsertSlice),
                    random_weight: 0.,
                    ordered_weight: 50.,
                },
                // WeightedMutation {
                //     mutation: InnerVectorMutation::InsertManyElements(insert_many_elements::InsertManyElements {
                //         nbr_added_elements: 4,
                //         repeated: true,
                //     }),
                //     random_weight: 6.,
                //     ordered_weight: 3.,
                // },
                // WeightedMutation {
                //     mutation: InnerVectorMutation::InsertManyElements(insert_many_elements::InsertManyElements {
                //         nbr_added_elements: 5,
                //         repeated: true,
                //     }),
                //     random_weight: 4.,
                //     ordered_weight: 2.,
                // },
            ],
        }
    }
}
