use crate::Mutator;
use std::{cmp::Ordering, marker::PhantomData};

pub struct AlternationMutator<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    pub mutators: Vec<M>,
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
    #[no_coverage]
    pub fn new(mutators: Vec<M>) -> Self {
        assert!(!mutators.is_empty());
        let complexity_from_choice = crate::mutators::size_to_cplxity(mutators.len());

        let max_complexity = mutators
            .iter()
            .map(
                #[no_coverage]
                |m| {
                    let max_cplx = m.max_complexity();
                    if max_cplx == 0. {
                        complexity_from_choice
                    } else {
                        max_cplx
                    }
                },
            )
            .max_by(
                #[no_coverage]
                |x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal),
            )
            .unwrap();
        let min_complexity = mutators
            .iter()
            .map(
                #[no_coverage]
                |m| {
                    let min_cplx = m.min_complexity();
                    if min_cplx == 0. {
                        complexity_from_choice
                    } else {
                        min_cplx
                    }
                },
            )
            .min_by(
                #[no_coverage]
                |x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal),
            )
            .unwrap();
        let complexity_from_choice = crate::mutators::size_to_cplxity(mutators.len());
        Self {
            mutators,
            complexity_from_choice,
            max_complexity,
            min_complexity,
            rng: fastrand::Rng::default(),
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
    mutator_idx: usize,
    inner: MS,
    arbitrary: AS,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cache<C> {
    inner: C,
    mutator_idx: usize,
}

pub enum UnmutateToken<T, U> {
    Replace(T),
    Inner(usize, U),
}

impl<T, M> AlternationMutator<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    #[no_coverage]
    fn default_mutation_step(
        &self,
        inner: M::MutationStep,
        idx: usize,
    ) -> MutationStep<M::MutationStep, <Self as Mutator<T>>::ArbitraryStep> {
        MutationStep {
            mutator_idx: idx,
            inner,
            arbitrary: {
                let mut step = self.default_arbitrary_step();
                step.indices.remove(idx);
                step
            },
        }
    }
}
impl<T, M> AlternationMutator<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    #[no_coverage]
    fn complexity_from_inner(&self, cplx: f64) -> f64 {
        if cplx == 0. {
            self.complexity_from_choice
        } else {
            cplx
        }
    }
}

impl<T, M> Mutator<T> for AlternationMutator<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    type Cache = Vec<Cache<M::Cache>>;
    type MutationStep = Vec<MutationStep<M::MutationStep, Self::ArbitraryStep>>;
    type ArbitraryStep = ArbitraryStep<M::ArbitraryStep>;
    type UnmutateToken = UnmutateToken<T, M::UnmutateToken>;

    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        Self::ArbitraryStep {
            inner: self
                .mutators
                .iter()
                .map(
                    #[no_coverage]
                    |m| m.default_arbitrary_step(),
                )
                .collect(),
            indices: (0..self.mutators.len()).collect(),
            idx: 0,
        }
    }

    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<(Self::Cache, Self::MutationStep)> {
        let mut caches = vec![];
        let mut steps = vec![];
        for (idx, mutator) in self.mutators.iter().enumerate() {
            if let Some((c, s)) = mutator.validate_value(value) {
                caches.push(Cache {
                    inner: c,
                    mutator_idx: idx,
                });
                steps.push(self.default_mutation_step(s, idx));
            }
        }
        if caches.is_empty() {
            return None;
        } else {
            Some((caches, steps))
        }
    }

    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.complexity_from_inner(self.max_complexity)
    }

    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.complexity_from_inner(self.min_complexity)
    }

    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        let cache = &cache[0];
        self.complexity_from_inner(self.mutators[cache.mutator_idx].complexity(value, &cache.inner))
    }

    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        if step.indices.is_empty() {
            return None;
        }
        if max_cplx < self.min_complexity() {
            return None;
        }

        let idx = step.indices[step.idx % step.indices.len()];
        let mutator = &self.mutators[idx];
        let inner_step = &mut step.inner[idx];
        if let Some((v, c)) = mutator.ordered_arbitrary(inner_step, max_cplx) {
            step.idx += 1;
            Some((v, self.complexity_from_inner(c)))
        } else {
            step.indices.remove(step.idx % step.indices.len());
            self.ordered_arbitrary(step, max_cplx)
        }
    }

    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        let idx = self.rng.usize(..self.mutators.len());
        let mutator = &self.mutators[idx];

        let (v, c) = mutator.random_arbitrary(max_cplx);
        (v, self.complexity_from_inner(c))
    }

    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        if step.is_empty() {
            return None;
        }
        if self.rng.usize(..100) == 0 {
            let (new_value, cplx) = self.random_arbitrary(max_cplx);
            let old_value = ::std::mem::replace(value, new_value);
            return Some((UnmutateToken::Replace(old_value), cplx));
        }

        let step_idx = self.rng.usize(..step.len());
        let chosen_step = &mut step[step_idx];
        let mutator_idx = chosen_step.mutator_idx;
        let chosen_cache = cache.iter_mut().find(|c| c.mutator_idx == mutator_idx).unwrap();

        let idx = chosen_cache.mutator_idx;
        assert_eq!(idx, mutator_idx);

        let mutator = &self.mutators[idx];
        if let Some((t, cplx)) =
            mutator.ordered_mutate(value, &mut chosen_cache.inner, &mut chosen_step.inner, max_cplx)
        {
            Some((UnmutateToken::Inner(idx, t), self.complexity_from_inner(cplx)))
        } else {
            if let Some((mut v, cplx)) = self.ordered_arbitrary(&mut chosen_step.arbitrary, max_cplx) {
                std::mem::swap(value, &mut v);
                Some((UnmutateToken::Replace(v), cplx))
            } else {
                step.remove(step_idx);
                if step.is_empty() {
                    None
                } else {
                    self.ordered_mutate(value, cache, step, max_cplx)
                }
            }
        }
    }

    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let cache_idx = self.rng.usize(..cache.len());
        let cache = &mut cache[cache_idx];

        let idx = cache.mutator_idx;
        let mutator = &self.mutators[idx];
        // this ensures that `arbitrary` is used instead of a unit mutator that will return
        // the same thing every time
        // there should be a better way to prevent this though
        // maybe it's time to give random_mutate a MutationStep too?
        if self.rng.usize(..100) == 0 || mutator.max_complexity() < 0.1 {
            let (new_value, cplx) = self.random_arbitrary(max_cplx);
            let old_value = ::std::mem::replace(value, new_value);
            return (UnmutateToken::Replace(old_value), cplx);
        }

        let (t, cplx) = mutator.random_mutate(value, &mut cache.inner, max_cplx);
        (UnmutateToken::Inner(idx, t), self.complexity_from_inner(cplx))
    }

    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateToken::Replace(v) => {
                let _ = std::mem::replace(value, v);
            }
            UnmutateToken::Inner(idx, t) => {
                let mutator = &self.mutators[idx];
                let cache = cache.iter_mut().find(|c| c.mutator_idx == idx).unwrap();
                mutator.unmutate(value, &mut cache.inner, t);
            }
        }
    }
}
