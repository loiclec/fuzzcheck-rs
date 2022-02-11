use crate::Mutator;
use std::{any::Any, cmp::Ordering, marker::PhantomData};

/**
A mutator that wraps multiple different mutators of the same type.

```
use fuzzcheck::mutators::alternation::AlternationMutator;
use fuzzcheck::mutators::integer_within_range::U8WithinRangeMutator;

let m1 = U8WithinRangeMutator::new(3 ..= 10);
let m2 = U8WithinRangeMutator::new(78 ..= 200);

let m = AlternationMutator::new(vec![m1, m2]);

// m will produce values either in 3..=10 or in 78..=200
```
*/
pub struct AlternationMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    mutators: Vec<M>,
    rng: fastrand::Rng,
    search_space_complexity: f64,
    added_complexity: f64,
    min_complexity: f64,
    max_complexity: f64,
    _phantom: PhantomData<T>,
}

impl<T, M> AlternationMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[no_coverage]
    pub fn new(mutators: Vec<M>, added_complexity: f64) -> Self {
        assert!(!mutators.is_empty());
        let complexity_from_choice = crate::mutators::size_to_cplxity(mutators.len());

        let search_space_complexity = mutators
            .iter()
            .map(
                #[no_coverage]
                |m| {
                    let cplx = m.global_search_space_complexity();
                    if cplx == 0. {
                        complexity_from_choice
                    } else {
                        cplx
                    }
                },
            )
            .max_by(
                #[no_coverage]
                |x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal),
            )
            .unwrap();

        let max_complexity = mutators
            .iter()
            .map(
                #[no_coverage]
                |m| m.max_complexity() + added_complexity,
            )
            .max_by(
                #[no_coverage]
                |x1, x2| x1.partial_cmp(x2).unwrap_or(Ordering::Equal),
            )
            .unwrap();
        let min_complexity = mutators
            .iter()
            .map(
                #[no_coverage]
                |m| m.min_complexity() + added_complexity,
            )
            .min_by(
                #[no_coverage]
                |x1, x2| x1.partial_cmp(x2).unwrap_or(Ordering::Equal),
            )
            .unwrap();

        Self {
            mutators,
            search_space_complexity,
            rng: fastrand::Rng::default(),
            added_complexity,
            min_complexity,
            max_complexity,
            _phantom: PhantomData,
        }
    }
}

#[doc(hidden)]
#[derive(Clone)]
pub struct ArbitraryStep<AS> {
    inner: Vec<AS>,
    indices: Vec<usize>,
    idx: usize,
}

#[doc(hidden)]
#[derive(Clone)]
pub struct MutationStep<MS, AS> {
    step: usize,
    mutator_idx: usize,
    inner: MS,
    arbitrary: AS,
}

#[doc(hidden)]
#[derive(Clone)]
pub struct Cache<C> {
    inner: C,
    mutator_idx: usize,
}

#[doc(hidden)]
pub enum UnmutateToken<T, U> {
    Replace(T),
    Inner(usize, U),
}

impl<T, M> AlternationMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[no_coverage]
    fn complexity_from_inner(&self, cplx: f64) -> f64 {
        cplx + self.added_complexity
    }
}

impl<T, M> Mutator<T> for AlternationMutator<T, M>
where
    T: Clone + 'static + 'static,
    M: Mutator<T>,
{
    #[doc(hidden)]
    type Cache = Vec<Cache<M::Cache>>;
    #[doc(hidden)]
    type MutationStep = Vec<MutationStep<M::MutationStep, Self::ArbitraryStep>>;
    #[doc(hidden)]
    type ArbitraryStep = ArbitraryStep<M::ArbitraryStep>;
    #[doc(hidden)]
    type UnmutateToken = UnmutateToken<T, M::UnmutateToken>;

    #[doc(hidden)]
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

    #[doc(hidden)]
    #[no_coverage]
    fn is_valid(&self, value: &T) -> bool {
        for m in self.mutators.iter() {
            if m.is_valid(value) {
                return true;
            }
        }
        false
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        let mut caches = vec![];
        for (idx, mutator) in self.mutators.iter().enumerate() {
            if let Some(c) = mutator.validate_value(value) {
                caches.push(Cache {
                    inner: c,
                    mutator_idx: idx,
                });
            }
        }
        if caches.is_empty() {
            None
        } else {
            Some(caches)
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        cache
            .iter()
            .map(
                #[no_coverage]
                |c| {
                    let m = &self.mutators[c.mutator_idx];
                    MutationStep {
                        step: 0,
                        mutator_idx: c.mutator_idx,
                        inner: m.default_mutation_step(value, &c.inner),
                        arbitrary: {
                            let mut step = self.default_arbitrary_step();
                            step.indices.remove(c.mutator_idx);
                            step
                        },
                    }
                },
            )
            .collect()
    }

    fn global_search_space_complexity(&self) -> f64 {
        self.search_space_complexity
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.max_complexity
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.min_complexity
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        let cache = &cache[0];
        self.complexity_from_inner(self.mutators[cache.mutator_idx].complexity(value, &cache.inner))
    }

    #[doc(hidden)]
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

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        let idx = self.rng.usize(..self.mutators.len());
        let mutator = &self.mutators[idx];

        let (v, c) = mutator.random_arbitrary(max_cplx);
        (v, self.complexity_from_inner(c))
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
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
        chosen_step.step += 1;
        // TODO: instead of 20, should be the sum of all important arbitraries of the sub mutators
        // and maybe it shouldn't be done all at the beginning, but be interspersed in between the
        // important mutations of the current mutator
        if chosen_step.step < 20 {
            if let Some((mut v, cplx)) = self.ordered_arbitrary(&mut chosen_step.arbitrary, max_cplx) {
                std::mem::swap(value, &mut v);
                return Some((UnmutateToken::Replace(v), cplx));
            }
        }

        let mutator_idx = chosen_step.mutator_idx;
        let chosen_cache = cache
            .iter_mut()
            .find(
                #[no_coverage]
                |c| c.mutator_idx == mutator_idx,
            )
            .unwrap();

        let idx = chosen_cache.mutator_idx;
        assert_eq!(idx, mutator_idx);

        let mutator = &self.mutators[idx];
        if let Some((t, cplx)) = mutator.ordered_mutate(
            value,
            &mut chosen_cache.inner,
            &mut chosen_step.inner,
            subvalue_provider,
            max_cplx,
        ) {
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
                    self.ordered_mutate(value, cache, step, subvalue_provider, max_cplx)
                }
            }
        }
    }

    #[doc(hidden)]
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
        // TODO: should use the global search space complexity here instead of max complexity?
        if self.rng.usize(..100) == 0 || mutator.max_complexity() < 0.1 {
            let (new_value, cplx) = self.random_arbitrary(max_cplx);
            let old_value = ::std::mem::replace(value, new_value);
            return (UnmutateToken::Replace(old_value), cplx);
        }

        let (t, cplx) = mutator.random_mutate(value, &mut cache.inner, max_cplx);
        (UnmutateToken::Inner(idx, t), self.complexity_from_inner(cplx))
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateToken::Replace(v) => {
                let _ = std::mem::replace(value, v);
            }
            UnmutateToken::Inner(idx, t) => {
                let mutator = &self.mutators[idx];
                let cache = cache
                    .iter_mut()
                    .find(
                        #[no_coverage]
                        |c| c.mutator_idx == idx,
                    )
                    .unwrap();
                mutator.unmutate(value, &mut cache.inner, t);
            }
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn visit_subvalues<'a>(&self, value: &'a T, cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        for cache in cache.iter() {
            let mutator_idx = cache.mutator_idx;
            let mutator = &self.mutators[mutator_idx];
            mutator.visit_subvalues(value, &cache.inner, visit);
        }
    }
}
