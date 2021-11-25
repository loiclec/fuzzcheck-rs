pub mod mutate_element;
pub mod remove;

use std::marker::PhantomData;
use std::ops::RangeInclusive;

use crate::{
    fenwick_tree::FenwickTree,
    mutators::{
        operations::{MutateOperation, RevertMutation},
        vose_alias::VoseAlias,
    },
    Mutator,
};

use crate::mutators::vector::VecArbitraryStep;
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq)]
pub struct RecursingPartIndex<RPI> {
    inner: Vec<RPI>,
    indices: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VecMutatorCache<C> {
    pub inner: Vec<C>,
    pub sum_cplx: f64,
    pub operation_alias: VoseAlias,
}

pub struct VecM<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    m: M,
    len_range: RangeInclusive<usize>,
    rng: fastrand::Rng,
    _phantom: PhantomData<T>,
}

pub enum Operation<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    Remove(remove::RemoveElement),
    Mutate(mutate_element::MutateElement<M::MutationStep>),
}
pub enum UnmutateToken<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    Remove(remove::RevertByInsertingElement<T>),
    Mutate(mutate_element::RevertByUnmutatingElement<M::UnmutateToken>),
}

pub struct MutationStep<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    operations: Vec<Operation<T, M>>,
    weights_and_times_chosen: Vec<(f64, f64)>,
    sampling: FenwickTree,
}
impl<T, M> VecM<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[no_coverage]
    fn complexity_from_inner(&self, cplx: f64, len: usize) -> f64 {
        1.0 + if cplx <= 0.0 { len as f64 } else { cplx }
    }
}
impl<T, M> Mutator<Vec<T>> for VecM<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type Cache = VecMutatorCache<M::Cache>;
    type MutationStep = MutationStep<T, M>;
    type ArbitraryStep = VecArbitraryStep;
    type UnmutateToken = UnmutateToken<T, M>;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        if self.m.max_complexity() == 0.0 {
            Self::ArbitraryStep::InnerMutatorIsUnit {
                length_step: *self.len_range.start(),
            }
        } else {
            Self::ArbitraryStep::Normal { make_empty: false }
        }
    }

    fn validate_value(&self, value: &Vec<T>) -> Option<Self::Cache> {
        let inner_caches: Vec<_> = value
            .iter()
            .map(
                #[no_coverage]
                |x| self.m.validate_value(x),
            )
            .collect::<Option<_>>()?;

        let cplxs = value
            .iter()
            .zip(inner_caches.iter())
            .map(
                #[no_coverage]
                |(v, c)| self.m.complexity(v, c),
            )
            .collect::<Vec<_>>();

        let sum_cplx = cplxs.iter().fold(
            0.0,
            #[no_coverage]
            |sum_cplx, c| sum_cplx + c,
        );

        let alias = VoseAlias::new(vec![1.0]);

        let cache = VecMutatorCache {
            inner: inner_caches,
            sum_cplx,
            operation_alias: alias,
        };
        Some(cache)
    }

    fn default_mutation_step(&self, value: &Vec<T>, cache: &Self::Cache) -> Self::MutationStep {
        let cplx = self.complexity(value, cache);
        let mut operations: Vec<Operation<T, M>> = vec![];
        let mut weights_and_times_chosen = vec![];
        if let Some(rm_op) = remove::RemoveElement::from_cache(self, value, &cache, cplx) {
            operations.push(Operation::Remove(rm_op));
            weights_and_times_chosen.push((100., 1.));
        }
        if let Some(m_op) = mutate_element::MutateElement::from_cache(self, value, &cache, cplx) {
            operations.push(Operation::Mutate(m_op));
            weights_and_times_chosen.push((1., 1.));
        }
        let probabilities = weights_and_times_chosen.iter().map(|(w, df)| w / df).collect();
        let sampling = FenwickTree::new(probabilities);
        MutationStep {
            operations,
            weights_and_times_chosen,
            sampling,
        }
    }

    fn max_complexity(&self) -> f64 {
        let max_len = *self.len_range.end();
        self.complexity_from_inner((max_len as f64) * self.m.max_complexity(), max_len.saturating_add(1))
    }

    fn min_complexity(&self) -> f64 {
        let min_len = *self.len_range.start();
        if min_len == 0 {
            return 1.0;
        } else {
            self.complexity_from_inner((min_len as f64) * self.m.min_complexity(), min_len)
        }
    }

    fn complexity(&self, value: &Vec<T>, cache: &Self::Cache) -> f64 {
        self.complexity_from_inner(cache.sum_cplx, value.len())
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(Vec<T>, f64)> {
        todo!()
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (Vec<T>, f64) {
        todo!()
    }

    fn ordered_mutate(
        &self,
        value: &mut Vec<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        let op_idx = step.sampling.sample(&self.rng)?;

        let op = &mut step.operations[op_idx];
        let token = match op {
            Operation::Remove(op) => op
                .apply(self, value, cache, max_cplx)
                .map(|(x, cplx)| (UnmutateToken::Remove(x), cplx)),
            Operation::Mutate(op) => op
                .apply(self, value, cache, max_cplx)
                .map(|(x, cplx)| (UnmutateToken::Mutate(x), cplx)),
        };
        if token.is_none() {
            step.weights_and_times_chosen.remove(op_idx);
            let probabilities = step.weights_and_times_chosen.iter().map(|(w, df)| w / df).collect();
            step.sampling = FenwickTree::new(probabilities);
            return self.ordered_mutate(value, cache, step, max_cplx);
        }

        let (weight, times_chosen) = &mut step.weights_and_times_chosen[op_idx];
        let old_score = *weight / *times_chosen;
        *times_chosen += 1.;
        let new_score = *weight / *times_chosen;
        let delta = new_score - old_score;

        step.sampling.update(op_idx, delta);

        token
    }

    // here I need random operations, they are different from ordered operations
    // they should also be selected from a fenwick tree, or even a vose alias
    // but they implement the same Operation trait
    // they are stored in the cache or the mutator insteed of the step
    // they shouldn't be mutated
    fn random_mutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        todo!()
    }

    fn unmutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateToken::Remove(t) => t.revert(self, value, cache),
            UnmutateToken::Mutate(t) => t.revert(self, value, cache),
        }
    }
    #[doc(hidden)]
    type RecursingPartIndex = RecursingPartIndex<M::RecursingPartIndex>;
    #[doc(hidden)]
    #[no_coverage]
    fn default_recursing_part_index(&self, value: &Vec<T>, cache: &Self::Cache) -> Self::RecursingPartIndex {
        RecursingPartIndex {
            inner: value
                .iter()
                .zip(cache.inner.iter())
                .map(|(v, c)| self.m.default_recursing_part_index(v, c))
                .collect(),
            indices: (0..value.len()).collect(),
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn recursing_part<'a, V, N>(
        &self,
        parent: &N,
        value: &'a Vec<T>,
        index: &mut Self::RecursingPartIndex,
    ) -> Option<&'a V>
    where
        V: Clone + 'static,
        N: Mutator<V>,
    {
        assert_eq!(index.inner.len(), index.indices.len());
        if index.inner.is_empty() {
            return None;
        }
        let choice = self.rng.usize(..index.inner.len());
        let subindex = &mut index.inner[choice];
        let value_index = index.indices[choice];
        let v = &value[value_index];
        let result = self.m.recursing_part(parent, v, subindex);
        if result.is_none() {
            index.inner.remove(choice);
            index.indices.remove(choice);
            self.recursing_part::<V, N>(parent, value, index)
        } else {
            result
        }
    }
}

// ---

impl<T, M> Clone for MutationStep<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    fn clone(&self) -> Self {
        Self {
            operations: self.operations.clone(),
            weights_and_times_chosen: self.weights_and_times_chosen.clone(),
            sampling: self.sampling.clone(),
        }
    }
}

//

impl<T, M> Clone for Operation<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    fn clone(&self) -> Self {
        match self {
            Self::Remove(arg0) => Self::Remove(arg0.clone()),
            Self::Mutate(arg0) => Self::Mutate(arg0.clone()),
        }
    }
}
