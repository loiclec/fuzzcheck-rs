use crate::{CrossoverArbitraryResult, Mutator, SubValueProvider};
use std::{
    any::{Any, TypeId},
    cmp::Ordering,
    collections::HashMap,
    marker::PhantomData,
};

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

#[doc(hidden)]
#[derive(Clone)]
pub struct ArbitraryStep<AS> {
    inner: Vec<AS>,
    indices: Vec<usize>,
    idx: usize,
}

#[doc(hidden)]
#[derive(Clone)]
pub struct RecursingPartIndex<RPI> {
    inner: Vec<RPI>,
    indices: Vec<usize>,
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

// impl<T, M> AlternationMutator<T, M>
// where
//     T: Clone,
//     M: Mutator<T>,
// {
//     #[no_coverage]
//     fn default_mutation_step(
//         &self,
//         inner: M::MutationStep,
//         idx: usize,
//     ) -> MutationStep<M::MutationStep, <Self as Mutator<T>>::ArbitraryStep> {
//         MutationStep {
//             step: 0,
//             mutator_idx: idx,
//             inner,
//             arbitrary: {
//                 let mut step = self.default_arbitrary_step();
//                 step.indices.remove(idx);
//                 step
//             },
//         }
//     }
// }
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
    T: Clone + 'static,
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

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.complexity_from_inner(self.max_complexity)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.complexity_from_inner(self.min_complexity)
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
    type RecursingPartIndex = RecursingPartIndex<M::RecursingPartIndex>;

    #[doc(hidden)]
    #[no_coverage]
    fn default_recursing_part_index(&self, value: &T, cache: &Self::Cache) -> Self::RecursingPartIndex {
        Self::RecursingPartIndex {
            inner: cache
                .iter()
                .map(
                    #[no_coverage]
                    |c| self.mutators[c.mutator_idx].default_recursing_part_index(value, &c.inner),
                )
                .collect(),
            indices: cache
                .iter()
                .map(
                    #[no_coverage]
                    |c| c.mutator_idx,
                )
                .collect(),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn recursing_part<'a, V, N>(&self, parent: &N, value: &'a T, index: &mut Self::RecursingPartIndex) -> Option<&'a V>
    where
        V: Clone + 'static,
        N: Mutator<V>,
    {
        // if vector is empty, return Nlne
        if index.inner.is_empty() {
            return None;
        }

        // choose a recursing part randomly
        let choice = self.rng.usize(..index.inner.len());
        let subindex = &mut index.inner[choice];
        let mutator_index = index.indices[choice];

        // call it
        let result = self.mutators[mutator_index].recursing_part::<V, N>(parent, value, subindex);

        // if none, remove it from the vector and call the function again
        if result.is_none() {
            index.inner.remove(choice);
            index.indices.remove(choice);
            self.recursing_part::<V, N>(parent, value, index)
        } else {
            result
        }
    }
    #[doc(hidden)]
    type LensPath = (usize, M::LensPath);
    #[doc(hidden)]
    #[no_coverage]
    fn lens<'a>(&self, value: &'a T, cache: &Self::Cache, path: &Self::LensPath) -> &'a dyn Any {
        let cache = &cache[path.0];
        let idx = cache.mutator_idx;
        let mutator = &self.mutators[idx];
        mutator.lens(value, &cache.inner, &path.1)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(&self, value: &T, cache: &Self::Cache) -> HashMap<TypeId, Vec<Self::LensPath>> {
        let mut result: HashMap<TypeId, Vec<Self::LensPath>> = HashMap::default();
        for (cache_idx, cache) in cache.iter().enumerate() {
            let mutator_idx = cache.mutator_idx;
            let mutator = &self.mutators[mutator_idx];
            let subpaths = mutator.all_paths(value, &cache.inner);
            for (type_id, subpaths) in subpaths {
                result
                    .entry(type_id)
                    .or_default()
                    .extend(subpaths.into_iter().map(|p| (cache_idx, p)));
            }
        }
        result
    }

    #[doc(hidden)]
    #[no_coverage]
    fn crossover_arbitrary(
        &self,
        subvalue_provider: &dyn SubValueProvider,
        max_cplx_from_crossover: f64,
        max_cplx: f64,
    ) -> CrossoverArbitraryResult<T> {
        let idx = self.rng.usize(..self.mutators.len());
        let mutator = &self.mutators[idx];

        let result = mutator.crossover_arbitrary(subvalue_provider, max_cplx_from_crossover, max_cplx);
        CrossoverArbitraryResult {
            value: result.value,
            complexity: self.complexity_from_inner(result.complexity),
            complexity_from_crossover: result.complexity_from_crossover,
        }
    }
}
