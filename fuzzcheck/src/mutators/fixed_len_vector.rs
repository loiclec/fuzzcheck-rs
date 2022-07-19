use std::any::Any;
use std::marker::PhantomData;

use fastrand::Rng;

use super::CrossoverStep;
use crate::{Mutator, CROSSOVER_RATE};

/// A mutator for vectors of a specific length
///
/// A different mutator is used for each element of the vector
pub struct FixedLenVecMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    pub rng: Rng,
    mutators: Vec<M>,
    min_complexity: f64,
    max_complexity: f64,
    search_space_complexity: f64,
    inherent_complexity: f64,
    _phantom: PhantomData<T>,
}
impl<T, M> FixedLenVecMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T> + Clone,
{
    #[no_coverage]
    pub fn new_with_repeated_mutator(mutator: M, len: usize) -> Self {
        Self::new(vec![mutator; len], true)
    }
}

impl<T, M> FixedLenVecMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[no_coverage]
    pub fn new(mutators: Vec<M>, inherent_complexity: bool) -> Self {
        assert!(!mutators.is_empty());

        // NOTE: this agrees with the vector mutator
        let inherent_complexity = if inherent_complexity {
            1.0 + if mutators[0].min_complexity() == 0.0 {
                mutators.len() as f64
            } else {
                0.0
            }
        } else {
            0.0
        };

        let max_complexity = mutators.iter().fold(
            0.0,
            #[no_coverage]
            |cplx, m| cplx + m.max_complexity(),
        ) + inherent_complexity;
        let min_complexity = mutators.iter().fold(
            0.0,
            #[no_coverage]
            |cplx, m| cplx + m.min_complexity(),
        ) + inherent_complexity;
        let search_space_complexity = mutators.iter().fold(
            0.0,
            #[no_coverage]
            |cplx, m| cplx + m.global_search_space_complexity(),
        );
        Self {
            rng: Rng::default(),
            mutators,
            min_complexity,
            max_complexity,
            search_space_complexity,
            inherent_complexity,
            _phantom: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct MutationStep<T, S> {
    inner: Vec<S>,
    element_step: usize,
    crossover_steps: Vec<CrossoverStep<T>>,
}

#[derive(Clone)]
pub struct VecMutatorCache<C> {
    inner: Vec<C>,
    sum_cplx: f64,
}
impl<C> Default for VecMutatorCache<C> {
    #[no_coverage]
    fn default() -> Self {
        Self {
            inner: Vec::new(),
            sum_cplx: 0.0,
        }
    }
}

pub enum UnmutateVecToken<T: Clone + 'static, M: Mutator<T>> {
    ReplaceElement(usize, T),
    Element(usize, M::UnmutateToken),
    Elements(Vec<(usize, M::UnmutateToken)>),
    Replace(Vec<T>),
}

impl<T: Clone + 'static, M: Mutator<T>> FixedLenVecMutator<T, M> {
    #[no_coverage]
    fn len(&self) -> usize {
        self.mutators.len()
    }
    #[no_coverage]
    fn mutate_elements(
        &self,
        value: &mut [T],
        cache: &mut VecMutatorCache<M::Cache>,
        idcs: &[usize],
        current_cplx: f64,
        max_cplx: f64,
    ) -> (UnmutateVecToken<T, M>, f64) {
        let mut cplx = current_cplx;
        let mut tokens = vec![];
        for &idx in idcs {
            let spare_cplx = max_cplx - cplx;
            let mutator = &self.mutators[idx];
            let el = &mut value[idx];
            let el_cache = &mut cache.inner[idx];

            let old_cplx = mutator.complexity(el, el_cache);

            let (token, new_cplx) = mutator.random_mutate(el, el_cache, spare_cplx + old_cplx);
            tokens.push((idx, token));
            cplx = cplx - old_cplx + new_cplx;
        }
        (UnmutateVecToken::Elements(tokens), cplx)
    }
    #[no_coverage]
    fn mutate_element(
        &self,
        value: &mut [T],
        cache: &mut VecMutatorCache<M::Cache>,
        step: &mut MutationStep<T, M::MutationStep>,
        subvalue_provider: &dyn crate::SubValueProvider,
        idx: usize,
        current_cplx: f64,
        spare_cplx: f64,
    ) -> Option<(UnmutateVecToken<T, M>, f64)> {
        let mutator = &self.mutators[idx];
        let el = &mut value[idx];
        let el_cache = &mut cache.inner[idx];
        let el_step = &mut step.inner[idx];

        let old_cplx = mutator.complexity(el, el_cache);

        if let Some((token, new_cplx)) =
            mutator.ordered_mutate(el, el_cache, el_step, subvalue_provider, spare_cplx + old_cplx)
        {
            Some((
                UnmutateVecToken::Element(idx, token),
                current_cplx - old_cplx + new_cplx,
            ))
        } else {
            None
        }
    }

    #[no_coverage]
    fn new_input_with_complexity(&self, target_cplx: f64) -> (Vec<T>, f64) {
        let mut v = Vec::with_capacity(self.len());
        let mut sum_cplx = 0.0;
        let mut remaining_cplx = target_cplx;
        let mut remaining_min_complexity = self.min_complexity();
        for (i, mutator) in self.mutators.iter().enumerate() {
            let mut max_cplx_element = (remaining_cplx / ((self.len() - i) as f64)) - remaining_min_complexity;
            let min_cplx_el = mutator.min_complexity();
            if min_cplx_el >= max_cplx_element {
                max_cplx_element = min_cplx_el;
            }
            let (x, x_cplx) = mutator.random_arbitrary(max_cplx_element);
            v.push(x);
            sum_cplx += x_cplx;
            remaining_cplx -= x_cplx;
            remaining_min_complexity -= mutator.min_complexity();
        }
        (v, sum_cplx)
    }
}

impl<T: Clone + 'static, M: Mutator<T>> Mutator<Vec<T>> for FixedLenVecMutator<T, M> {
    #[doc(hidden)]
    type Cache = VecMutatorCache<M::Cache>;
    #[doc(hidden)]
    type MutationStep = MutationStep<T, M::MutationStep>;
    #[doc(hidden)]
    type ArbitraryStep = ();
    #[doc(hidden)]
    type UnmutateToken = UnmutateVecToken<T, M>;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {}

    #[doc(hidden)]
    #[no_coverage]
    fn is_valid(&self, value: &Vec<T>) -> bool {
        if value.len() != self.mutators.len() {
            return false;
        }
        for (m, v) in self.mutators.iter().zip(value.iter()) {
            if !m.is_valid(v) {
                return false;
            }
        }
        true
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &Vec<T>) -> Option<Self::Cache> {
        if value.len() != self.mutators.len() {
            return None;
        }
        let inner_caches: Vec<_> = value
            .iter()
            .zip(self.mutators.iter())
            .map(
                #[no_coverage]
                |(x, mutator)| mutator.validate_value(x),
            )
            .collect::<Option<_>>()?;

        let sum_cplx = value.iter().zip(self.mutators.iter()).zip(inner_caches.iter()).fold(
            0.0,
            #[no_coverage]
            |cplx, ((v, mutator), cache)| cplx + mutator.complexity(v, cache),
        );

        let cache = VecMutatorCache {
            inner: inner_caches,
            sum_cplx,
        };
        Some(cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, value: &Vec<T>, cache: &Self::Cache) -> Self::MutationStep {
        let inner = value
            .iter()
            .zip(cache.inner.iter())
            .zip(self.mutators.iter())
            .map(
                #[no_coverage]
                |((v, c), m)| m.default_mutation_step(v, c),
            )
            .collect::<Vec<_>>();
        MutationStep {
            inner,
            element_step: 0,
            crossover_steps: vec![CrossoverStep::default(); value.len()],
        }
    }

    #[doc(hidden)]
    #[no_coverage]
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
    fn complexity(&self, _value: &Vec<T>, cache: &Self::Cache) -> f64 {
        cache.sum_cplx + self.inherent_complexity
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, _step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(Vec<T>, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        Some(self.random_arbitrary(max_cplx))
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (Vec<T>, f64) {
        let target_cplx = crate::mutators::gen_f64(&self.rng, 1.0..max_cplx);
        let (v, sum_cplx) = self.new_input_with_complexity(target_cplx);
        (v, sum_cplx + self.inherent_complexity)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut Vec<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        if value.is_empty() || self.rng.usize(0..100) == 0 {
            let (mut v, cplx) = self.random_arbitrary(max_cplx);
            std::mem::swap(value, &mut v);
            return Some((UnmutateVecToken::Replace(v), cplx));
        }
        if self.rng.u8(..CROSSOVER_RATE) == 0 {
            let choice = self.rng.usize(..value.len());
            let step = &mut step.crossover_steps[choice];
            let old_el_cplx = self.mutators[choice].complexity(&value[choice], &cache.inner[choice]);
            let current_cplx = self.complexity(value, cache);
            let max_el_cplx = current_cplx - old_el_cplx - self.inherent_complexity;
            if let Some((el, new_el_cplx)) = step.get_next_subvalue(subvalue_provider, max_el_cplx) && self.mutators[choice].is_valid(el) {
                let mut el = el.clone();
                std::mem::swap(&mut value[choice], &mut el);
                let cplx = cache.sum_cplx - old_el_cplx + new_el_cplx + self.inherent_complexity;
                let token = UnmutateVecToken::ReplaceElement(choice, el);
                return Some((token, cplx));
            }
        }
        let current_cplx = self.complexity(value, cache);
        if value.len() > 1 && self.rng.usize(..20) == 0 {
            let mut idcs = (0..value.len()).collect::<Vec<_>>();
            self.rng.shuffle(&mut idcs);
            let count = self.rng.usize(2..=value.len());
            let idcs = &idcs[..count];
            Some(self.mutate_elements(value, cache, idcs, current_cplx, max_cplx))
        } else {
            let spare_cplx = max_cplx - current_cplx - self.inherent_complexity;
            let idx = step.element_step % value.len();
            step.element_step += 1;
            self.mutate_element(value, cache, step, subvalue_provider, idx, current_cplx, spare_cplx)
                .or_else(
                    #[no_coverage]
                    || Some(self.random_mutate(value, cache, max_cplx)),
                )
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        if value.is_empty() || self.rng.usize(0..100) == 0 {
            let (mut v, cplx) = self.random_arbitrary(max_cplx);
            std::mem::swap(value, &mut v);
            return (UnmutateVecToken::Replace(v), cplx);
        }
        let current_cplx = self.complexity(value, cache);
        if value.len() > 1 && self.rng.usize(..20) == 0 {
            let mut idcs = (0..value.len()).collect::<Vec<_>>();
            self.rng.shuffle(&mut idcs);
            let count = self.rng.usize(2..=value.len());
            let idcs = &idcs[..count];
            return self.mutate_elements(value, cache, idcs, current_cplx, max_cplx);
        }
        let spare_cplx = max_cplx - current_cplx;

        let idx = self.rng.usize(0..value.len());
        let el = &mut value[idx];
        let el_cache = &mut cache.inner[idx];

        let old_el_cplx = self.mutators[idx].complexity(el, el_cache);
        let (token, new_el_cplx) = self.mutators[idx].random_mutate(el, el_cache, spare_cplx + old_el_cplx);

        (
            UnmutateVecToken::Element(idx, token),
            current_cplx - old_el_cplx + new_el_cplx,
        )
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateVecToken::Element(idx, inner_t) => {
                let el = &mut value[idx];
                self.mutators[idx].unmutate(el, &mut cache.inner[idx], inner_t);
            }
            UnmutateVecToken::Elements(tokens) => {
                for (idx, token) in tokens {
                    let el = &mut value[idx];
                    self.mutators[idx].unmutate(el, &mut cache.inner[idx], token);
                }
            }
            UnmutateVecToken::Replace(new_value) => {
                let _ = std::mem::replace(value, new_value);
            }
            UnmutateVecToken::ReplaceElement(idx, el) => {
                let _ = std::mem::replace(&mut value[idx], el);
            }
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn visit_subvalues<'a>(&self, value: &'a Vec<T>, cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        if !value.is_empty() {
            for idx in 0..value.len() {
                let cplx = self.mutators[idx].complexity(&value[idx], &cache.inner[idx]);
                visit(&value[idx], cplx);
            }
            for ((el, el_cache), mutator) in value.iter().zip(cache.inner.iter()).zip(self.mutators.iter()) {
                mutator.visit_subvalues(el, el_cache, visit);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::FixedLenVecMutator;
    use crate::mutators::integer::U8Mutator;
    use crate::Mutator;
    #[test]
    #[no_coverage]
    fn test_constrained_length_mutator() {
        let m = FixedLenVecMutator::<u8, U8Mutator>::new_with_repeated_mutator(U8Mutator::default(), 3);
        for _ in 0..100 {
            let (x, _) = m.ordered_arbitrary(&mut (), 800.0).unwrap();
            eprintln!("{:?}", x);
        }
    }
}
