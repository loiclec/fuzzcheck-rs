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
impl<T, M> AlternationMutator<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    #[no_coverage]
    fn complexity_from_inner(&self, cplx: f64) -> f64 {
        self.complexity_from_choice + cplx
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
        None
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
        let max_cplx = max_cplx - self.complexity_from_choice;

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
        let max_cplx = max_cplx - self.complexity_from_choice;

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
        let max_cplx = max_cplx - self.complexity_from_choice;

        if self.rng.usize(..100) == 0 {
            let (new_value, cplx) = self.random_arbitrary(max_cplx);
            let old_value = ::std::mem::replace(value, new_value);
            return Some((UnmutateToken::Replace(old_value), cplx));
        }

        let idx = cache.mutator_idx;
        let mutator = &self.mutators[idx];
        if let Some((t, cplx)) = mutator.ordered_mutate(value, &mut cache.inner, &mut step.inner, max_cplx) {
            Some((UnmutateToken::Inner(idx, t), self.complexity_from_inner(cplx)))
        } else if let Some((mut v, cplx)) = self.ordered_arbitrary(&mut step.arbitrary, max_cplx) {
            std::mem::swap(value, &mut v);
            Some((UnmutateToken::Replace(v), cplx))
        } else {
            None
        }
    }

    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let idx = cache.mutator_idx;
        let mutator = &self.mutators[idx];
        let max_cplx = max_cplx - self.complexity_from_choice;
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
                mutator.unmutate(value, &mut cache.inner, t);
            }
        }
    }
}
