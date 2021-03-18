use std::{cmp::Ordering, marker::PhantomData};

use fuzzcheck_traits::Mutator;

use crate::{
    algebra::{CommonMutatorSuperType, MutatorSuperType},
};

#[macro_export]
macro_rules! alternation_mutator {
    ( $first: expr , $( $x:expr ),* $(,)?) => {
        {
            let result = $crate::alternation::AlternationMutator::new(vec![$first]);
            $(
                let result = result.adding_mutator($x);
            )*
            result
        }
    };
}

pub struct AlternationMutator<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    mutators: Vec<M>,
    complexity_from_choice: f64,
    max_complexity: f64,
    min_complexity: f64,
    rng: fastrand::Rng,
    _phantom: PhantomData<T>,
}

impl<T, M> AlternationMutator<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    pub fn new(mutators: Vec<M>) -> Self {
        assert!(!mutators.is_empty());
        let max_complexity = mutators
            .iter()
            .map(|m| m.max_complexity())
            .max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
            .unwrap();
        let min_complexity = mutators
            .iter()
            .map(|m| m.max_complexity())
            .min_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
            .unwrap();
        let complexity_from_choice = crate::size_to_cplxity(mutators.len());
        Self {
            mutators,
            complexity_from_choice,
            max_complexity,
            min_complexity,
            rng: fastrand::Rng::default(),
            _phantom: PhantomData,
        }
    }

    pub fn adding_mutator<N>(self, mutator: N) -> AlternationMutator<T, <M as CommonMutatorSuperType<T, N>>::Output>
    where
        N: Mutator<T>,
        M: CommonMutatorSuperType<T, N>,
    {
        let mut mutators = self
            .mutators
            .into_iter()
            .map(|m| <<M as CommonMutatorSuperType<T, N>>::Output as MutatorSuperType<T, M>>::upcast(m))
            .collect::<Vec<_>>();

        let mut max_complexity = self.max_complexity;
        let mut min_complexity = self.min_complexity;
        let mutator = <<M as CommonMutatorSuperType<T, N>>::Output as MutatorSuperType<T, N>>::upcast(mutator);
        mutators.push(mutator);
        let mutator_max_cplx = mutator.max_complexity();
        if max_complexity < mutator_max_cplx {
            max_complexity = mutator_max_cplx;
        }
        let mutator_min_cplx = mutator.min_complexity();
        if min_complexity > mutator_min_cplx {
            min_complexity = mutator_min_cplx;
        }
        let complexity_from_choice = crate::size_to_cplxity(mutators.len());
        AlternationMutator {
            mutators,
            complexity_from_choice,
            max_complexity,
            min_complexity,
            rng: self.rng,
            _phantom: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct ArbitraryStep<AS> {
    inner: Vec<AS>,
    indices: Vec<usize>,
    idx: usize,
}

#[derive(Clone)]
pub struct MutationStep<MS, AS> {
    inner: MS,
    arbitrary: AS,
}

#[derive(Clone)]
pub struct Cache<C> {
    inner: C,
    mutator_idx: usize,
}

enum UnmutateToken<T, C, U> {
    Replace(T, C),
    Inner(U),
}

impl<T, M> AlternationMutator<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    fn default_mutation_step(&self, inner: M::MutationStep, idx: usize) -> <Self as Mutator<T>>::MutationStep {
        MutationStep {
            inner,
            arbitrary: {
                let mut step = self.default_arbitrary_step();
                step.indices.remove(idx);
                step
            },
        }
    }
}
impl<T, M> Mutator<T> for AlternationMutator<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    type Cache = Cache<M::Cache>;
    type MutationStep = MutationStep<M::MutationStep, Self::ArbitraryStep>;
    type ArbitraryStep = ArbitraryStep<M::ArbitraryStep>;
    type UnmutateToken = UnmutateToken<T, Self::Cache, M::UnmutateToken>;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        Self::ArbitraryStep {
            inner: self.mutators.iter().map(|m| m.default_arbitrary_step()).collect(),
            indices: (0..self.mutators.len()).collect(),
            idx: 0,
        }
    }

    fn validate_value(&self, value: &T) -> Option<(Self::Cache, Self::MutationStep)> {
        for (idx, mutator) in self.mutators.iter().enumerate() {
            if let Some((c, s)) = mutator.validate_value(value) {
                return Some((
                    Cache {
                        inner: c,
                        mutator_idx: idx,
                    },
                    self.default_mutation_step(s, idx),
                ));
            }
        }
        return None;
    }

    fn max_complexity(&self) -> f64 {
        self.complexity_from_choice + self.max_complexity
    }

    fn min_complexity(&self) -> f64 {
        self.complexity_from_choice + self.min_complexity
    }

    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.complexity_from_choice + self.mutators[cache.mutator_idx].complexity(value, &cache.inner)
    }

    fn ordered_arbitrary(
        &self,
        step: &mut Self::ArbitraryStep,
        max_cplx: f64,
    ) -> Option<(T, Self::Cache, Self::MutationStep)> {
        if step.indices.is_empty() {
            return None;
        }
        let idx = step.indices[step.idx % step.indices.len()];
        let mutator = self.mutators[idx];
        let inner_step = &mut step.inner[idx];
        if let Some((v, c, s)) = mutator.ordered_arbitrary(inner_step, max_cplx) {
            step.idx += 1;
            Some((
                v,
                Cache {
                    inner: c,
                    mutator_idx: idx,
                },
                self.default_mutation_step(s, idx),
            ))
        } else {
            step.indices.remove(idx);
            self.ordered_arbitrary(step, max_cplx)
        }
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (T, Self::Cache, Self::MutationStep) {
        let idx = self.rng.usize(..self.mutators.len());
        let mutator = self.mutators[idx];
        let (v, c, s) = mutator.random_arbitrary(max_cplx);
        (
            v,
            Cache {
                inner: c,
                mutator_idx: idx,
            },
            self.default_mutation_step(s, idx),
        )
    }

    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        let idx = cache.mutator_idx;
        let mutator = self.mutators[idx];
        if let Some(t) = mutator.ordered_mutate(value, &mut cache.inner, &mut step.inner, max_cplx) {
            Some(UnmutateToken::Inner(t))
        } else {
            if let Some((mut v, mut c, s)) = self.ordered_arbitrary(&mut step.arbitrary, max_cplx) {
                std::mem::swap(value, &mut v);
                std::mem::swap(cache, &mut c);
                return Some(UnmutateToken::Replace(v, c));
            } else {
                return None;
            }
        }
    }

    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        let idx = cache.mutator_idx;
        let mutator = self.mutators[idx];
        // TODO: randomly create new from arbitrary
        let t = mutator.random_mutate(value, &mut cache.inner, max_cplx);
        UnmutateToken::Inner(t)
    }

    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateToken::Replace(v, c) => {
                std::mem::swap(value, &mut v);
                std::mem::swap(cache, &mut c);
            }
            UnmutateToken::Inner(t) => {
                let mutator = self.mutators[cache.mutator_idx];
                mutator.unmutate(value, &mut cache.inner, t);
            }
        }
    }
}
