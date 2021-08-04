use std::marker::PhantomData;

use crate::Mutator;
use fastrand::Rng;

pub struct FixedLenVecMutator<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    pub rng: Rng,
    mutators: Vec<M>,
    min_complexity: f64,
    max_complexity: f64,
    _phantom: PhantomData<T>,
}
impl<T, M> FixedLenVecMutator<T, M>
where
    T: Clone,
    M: Mutator<T> + Clone,
{
    #[no_coverage]
    pub fn new_with_repeated_mutator(mutator: M, len: usize) -> Self {
        Self::new(std::iter::repeat(mutator).take(len).collect())
    }
}

impl<T, M> FixedLenVecMutator<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    #[no_coverage]
    pub fn new(mutators: Vec<M>) -> Self {
        assert!(!mutators.is_empty());
        let max_complexity = crate::mutators::size_to_cplxity(mutators.len() + 1)
            + mutators.iter().fold(0.0, |cplx, m| cplx + m.max_complexity());
        let min_complexity = crate::mutators::size_to_cplxity(mutators.len() + 1)
            + mutators.iter().fold(0.0, |cplx, m| cplx + m.min_complexity());
        Self {
            rng: Rng::default(),
            mutators,
            min_complexity,
            max_complexity,
            _phantom: PhantomData,
        }
    }
}

pub struct MutationStep<S> {
    inner: Vec<S>,
    element_step: usize,
}

#[derive(Clone, PartialEq)]
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

pub enum UnmutateVecToken<T: Clone, M: Mutator<T>> {
    Element(usize, M::UnmutateToken),
    Replace(Vec<T>),
}

impl<T: Clone, M: Mutator<T>> FixedLenVecMutator<T, M> {
    #[no_coverage]
    fn len(&self) -> usize {
        self.mutators.len()
    }
    #[no_coverage]
    fn mutate_element(
        &self,
        value: &mut Vec<T>,
        cache: &mut VecMutatorCache<M::Cache>,
        step: &mut MutationStep<M::MutationStep>,
        idx: usize,
        current_cplx: f64,
        spare_cplx: f64,
    ) -> Option<(UnmutateVecToken<T, M>, f64)> {
        let mutator = &self.mutators[idx];
        let el = &mut value[idx];
        let el_cache = &mut cache.inner[idx];
        let el_step = &mut step.inner[idx];

        let old_cplx = mutator.complexity(el, el_cache);

        if let Some((token, new_cplx)) = mutator.ordered_mutate(el, el_cache, el_step, spare_cplx + old_cplx) {
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
            let mut max_cplx_element = remaining_cplx / ((self.len() - i) as f64) - remaining_min_complexity;
            let min_cplx_el = mutator.min_complexity();
            if min_cplx_el >= max_cplx_element {
                max_cplx_element = min_cplx_el;
            }
            let cplx_element = crate::mutators::gen_f64(&self.rng, min_cplx_el..max_cplx_element);
            let (x, x_cplx) = mutator.random_arbitrary(cplx_element);
            v.push(x);
            sum_cplx += x_cplx;
            remaining_cplx -= x_cplx;
            remaining_min_complexity -= mutator.min_complexity();
        }
        (v, self.min_complexity + sum_cplx)
    }
}

impl<T: Clone, M: Mutator<T>> Mutator<Vec<T>> for FixedLenVecMutator<T, M> {
    type Cache = VecMutatorCache<M::Cache>;
    type MutationStep = MutationStep<M::MutationStep>;
    type ArbitraryStep = ();
    type UnmutateToken = UnmutateVecToken<T, M>;

    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {}

    #[no_coverage]
    fn validate_value(&self, value: &Vec<T>) -> Option<(Self::Cache, Self::MutationStep)> {
        let inner: Vec<_> = value
            .iter()
            .zip(self.mutators.iter())
            .filter_map(|(x, mutator)| mutator.validate_value(x))
            .collect();

        if inner.len() < value.len() {
            return None;
        }

        let mut inner_caches = Vec::with_capacity(inner.len());
        let mut inner_steps = Vec::with_capacity(inner.len());
        for (cache, step) in inner.into_iter() {
            inner_caches.push(cache);
            inner_steps.push(step);
        }
        let sum_cplx = value
            .iter()
            .zip(self.mutators.iter())
            .zip(inner_caches.iter())
            .fold(0.0, |cplx, ((v, mutator), cache)| cplx + mutator.complexity(v, cache));

        let cache = VecMutatorCache {
            inner: inner_caches,
            sum_cplx,
        };
        let step = MutationStep {
            inner: inner_steps,
            element_step: 0,
        };

        Some((cache, step))
    }

    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.max_complexity
    }

    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.min_complexity
    }

    #[no_coverage]
    fn complexity(&self, _value: &Vec<T>, cache: &Self::Cache) -> f64 {
        self.min_complexity + cache.sum_cplx
    }

    #[no_coverage]
    fn ordered_arbitrary(&self, _step: &mut Self::ArbitraryStep, mut max_cplx: f64) -> Option<(Vec<T>, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }
        Some(self.random_arbitrary(max_cplx))
    }

    #[no_coverage]
    fn random_arbitrary(&self, mut max_cplx: f64) -> (Vec<T>, f64) {
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }

        let target_cplx = crate::mutators::gen_f64(&self.rng, 1.0..max_cplx);

        self.new_input_with_complexity(target_cplx)
    }

    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut Vec<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        mut max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        if self.rng.usize(0..100) == 0 {
            let (mut v, cplx) = self.random_arbitrary(max_cplx);
            std::mem::swap(value, &mut v);
            return Some((UnmutateVecToken::Replace(v), cplx));
        }
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }
        let current_cplx = self.complexity(value, cache);
        let spare_cplx = max_cplx - current_cplx;

        let idx = step.element_step % value.len();
        step.element_step += 1;
        self.mutate_element(value, cache, step, idx, current_cplx, spare_cplx)
            .or_else(|| Some(self.random_mutate(value, cache, max_cplx)))
    }

    #[no_coverage]
    fn random_mutate(
        &self,
        value: &mut Vec<T>,
        cache: &mut Self::Cache,
        mut max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }
        if self.rng.usize(0..100) == 0 {
            let (mut v, cplx) = self.random_arbitrary(max_cplx);
            std::mem::swap(value, &mut v);
            return (UnmutateVecToken::Replace(v), cplx);
        }
        let current_cplx = self.complexity(value, cache);
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

    #[no_coverage]
    fn unmutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateVecToken::Element(idx, inner_t) => {
                let el = &mut value[idx];
                self.mutators[idx].unmutate(el, &mut cache.inner[idx], inner_t);
            }
            UnmutateVecToken::Replace(new_value) => {
                let _ = std::mem::replace(value, new_value);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::Mutator;

    use super::FixedLenVecMutator;
    use crate::mutators::integer::U8Mutator;
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
